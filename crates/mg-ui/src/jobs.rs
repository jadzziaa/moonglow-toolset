//! Jobs: long work done off the interface's thread, so that the window
//! keeps drawing, says how far the work is, and can call it off.
//!
//! A job reads the module as it was when it began (a snapshot: resources
//! are shared, not copied) and the game data, on a thread of its own, and
//! hands back what it made. Then, on the interface's thread, its `finish`
//! puts that into the application: edits through [`Moonglow::apply`], lines
//! in the log, a window opened. While a job runs the window takes no other
//! input, so nothing changes under it and nothing needs reconciling after.
//!
//! One job runs at a time. With [`Moonglow::background_jobs`] off (tests,
//! and anything without a window to keep alive), starting a job waits for
//! it: the same thread, the same hand-over, done before `start_job` returns.

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use mg_module::Module;
use mg_rules::GameData;

use crate::Moonglow;

// A job's thread reads the game data while the interface draws from it.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<GameData>();
    shared::<Module>();
};

/// How far a job is, and whether it was called off; the job writes, the
/// window reads.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
    cancel: AtomicBool,
    note: Mutex<String>,
}

impl Progress {
    /// `done` of `total` steps are behind (a total of 0: not known).
    pub fn step(&self, done: usize, total: usize) {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }

    /// What the job is doing now, in a few words.
    pub fn say(&self, note: impl Into<String>) {
        *self.note.lock().unwrap_or_else(|e| e.into_inner()) = note.into();
    }

    /// Whether the job was called off: it should stop at its next step and
    /// return (what it returns is dropped).
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn note(&self) -> String {
        self.note.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// What a job reads.
#[derive(Debug)]
pub struct Context {
    /// The module as it was when the job began, unsaved changes included.
    pub module: Module,
    /// The game data, the module layered in as it was then.
    pub game: Option<Arc<GameData>>,
    pub progress: Arc<Progress>,
}

type Made = Box<dyn Any + Send>;
type Finish = Box<dyn FnOnce(&mut Moonglow, Made)>;

/// A job under way.
pub struct Job {
    pub title: String,
    progress: Arc<Progress>,
    thread: Option<JoinHandle<Made>>,
    finish: Option<Finish>,
    started: Instant,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job").field("title", &self.title).finish_non_exhaustive()
    }
}

/// What a thread that panicked said.
fn panic_text(panic: &Made) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "it stopped unexpectedly".into())
}

impl Moonglow {
    /// Whether a job is under way (the window takes no other input then).
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Starts `work` on a thread of its own, on the open module as it is
    /// now; when it is done, `finish` gets what it made, on the interface's
    /// thread. Nothing starts without an open module, or while another job
    /// runs (the log says so).
    pub(crate) fn start_job<T: Send + 'static>(
        &mut self,
        title: impl Into<String>,
        work: impl FnOnce(&Context) -> T + Send + 'static,
        finish: impl FnOnce(&mut Moonglow, T) + 'static,
    ) {
        let title = title.into();
        if let Some(job) = &self.job {
            self.log.warn(format!("{title}: waiting for {} to finish", job.title));
            return;
        }
        // The module's layer of the game data as the module is now (which
        // changes the game data: before the job shares it).
        self.refresh_module_layer();
        let module = match self.ws.as_mut().map(|ws| ws.snapshot()) {
            Some(Ok(m)) => m,
            Some(Err(e)) => {
                self.log.error(format!("{title}: {e}"));
                return;
            }
            None => {
                self.log.error(format!("{title} needs an open module"));
                return;
            }
        };
        let progress = Arc::new(Progress::default());
        let context = Context { module, game: self.game.clone(), progress: progress.clone() };
        // (The context, and its share of the game data, is dropped when
        // the work returns: before `finish` may change the game data.)
        let thread = std::thread::Builder::new()
            .name("moonglow job".into())
            .spawn(move || -> Made { Box::new(work(&context)) });
        let thread = match thread {
            Ok(t) => t,
            Err(e) => {
                self.log.error(format!("{title}: could not start: {e}"));
                return;
            }
        };
        self.job = Some(Job {
            title,
            progress,
            thread: Some(thread),
            finish: Some(Box::new(move |app, made| {
                if let Ok(made) = made.downcast::<T>() {
                    finish(app, *made);
                }
            })),
            started: Instant::now(),
        });
        if !self.background_jobs {
            self.finish_job();
        }
    }

    /// Each frame: hands a finished job's work over; else asks to be drawn
    /// again soon.
    pub(crate) fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        if job.thread.as_ref().is_some_and(|t| t.is_finished()) {
            self.finish_job();
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    /// Waits for the job's thread and hands its work over (or says that it
    /// was called off, or failed).
    fn finish_job(&mut self) {
        let Some(mut job) = self.job.take() else { return };
        let Some(thread) = job.thread.take() else { return };
        match thread.join() {
            Ok(_) if job.progress.cancelled() => self.log.info(format!("{}: canceled", job.title)),
            Ok(made) => {
                if let Some(finish) = job.finish.take() {
                    finish(self, made);
                }
            }
            Err(panic) => {
                self.log.error(format!("{} failed: {}", job.title, panic_text(&panic)));
            }
        }
    }
}

/// The running job's window: its name, how far it is, and Cancel. Nothing
/// else takes input while it shows.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(job) = &app.job else { return };
    egui::Modal::new(egui::Id::new("job")).show(ctx, |ui| {
        ui.set_width(320.0);
        ui.heading(&job.title);
        let (done, total) =
            (job.progress.done.load(Ordering::Relaxed), job.progress.total.load(Ordering::Relaxed));
        if total > 0 {
            let bar = egui::ProgressBar::new(done as f32 / total as f32)
                .text(format!("{done} of {total}"));
            ui.add(bar);
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("{} s", job.started.elapsed().as_secs()));
            });
        }
        let note = job.progress.note();
        if !note.is_empty() {
            ui.label(note);
        }
        ui.add_space(6.0);
        if job.progress.cancelled() {
            ui.weak("Canceling…");
        } else if ui.button("Cancel").clicked() {
            job.progress.cancel.store(true, Ordering::Relaxed);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use mg_module::ModuleLocation;

    use super::*;
    use crate::Level;

    /// An application with a small module open, its jobs in the background.
    fn app(name: &str) -> Moonglow {
        let dir = mg_testkit::scratch_dir(name);
        let mut m = Module::new();
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        m.set(key, b"void main() {}".to_vec());
        m.save_as(&ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.open_module(&dir.join("m.mod"));
        app.background_jobs = true;
        app
    }

    /// Frames until the job is over (or a test has hung).
    fn until_done(app: &mut Moonglow) {
        let ctx = egui::Context::default();
        let started = Instant::now();
        while app.busy() {
            app.poll_job(&ctx);
            std::thread::sleep(Duration::from_millis(5));
            assert!(started.elapsed() < Duration::from_secs(20), "the job never ended");
        }
    }

    fn logged(app: &Moonglow, level: Level, what: &str) -> bool {
        app.log.entries.iter().any(|(l, m)| *l == level && m.contains(what))
    }

    #[test]
    fn a_job_works_on_its_own_thread_and_hands_over_on_the_interface_s() {
        let mut app = app("ui-job");
        let (go, wait) = mpsc::channel::<()>();
        let here = std::thread::current().id();
        app.start_job(
            "Count",
            move |job| {
                job.progress.step(1, 2);
                job.progress.say("counting");
                wait.recv().ok();
                (std::thread::current().id(), job.module.len())
            },
            move |app, (thread, resources)| {
                assert_ne!(thread, here, "the work ran elsewhere");
                assert_eq!(std::thread::current().id(), here, "the hand-over runs here");
                app.log.info(format!("counted {resources}"));
            },
        );
        // Under way: the window draws, and nothing else starts.
        let ctx = egui::Context::default();
        app.poll_job(&ctx);
        assert!(app.busy() && !logged(&app, Level::Info, "counted"));
        app.start_job("Another", |_| (), |app, ()| app.log.info("another ran"));
        assert!(logged(&app, Level::Warning, "Another: waiting for Count to finish"));
        let progress = app.job.as_ref().unwrap().progress.clone();
        while progress.note() != "counting" {
            std::thread::yield_now();
        }
        assert_eq!(progress.done.load(Ordering::Relaxed), 1);
        go.send(()).unwrap();
        until_done(&mut app);
        assert!(logged(&app, Level::Info, "counted 1"), "{:?}", app.log.entries);
        assert!(!logged(&app, Level::Info, "another ran"));
    }

    #[test]
    fn a_job_called_off_hands_nothing_over() {
        let mut app = app("ui-job-cancel");
        let (go, wait) = mpsc::channel::<()>();
        app.start_job(
            "Slow",
            move |job| {
                wait.recv().ok();
                job.progress.cancelled()
            },
            |app, _| app.log.info("handed over"),
        );
        app.job.as_ref().unwrap().progress.cancel.store(true, Ordering::Relaxed);
        go.send(()).unwrap();
        until_done(&mut app);
        assert!(logged(&app, Level::Info, "Slow: canceled"));
        assert!(!logged(&app, Level::Info, "handed over"));
    }

    #[test]
    fn a_job_that_fails_says_so_and_the_application_goes_on() {
        let mut app = app("ui-job-panic");
        app.start_job(
            "Broken",
            |_| -> u8 { panic!("no such thing") },
            |app, _| app.log.info("handed over"),
        );
        until_done(&mut app);
        assert!(logged(&app, Level::Error, "Broken failed: no such thing"));
        // The next one runs; the game data (none here) and module are whole.
        app.start_job("Next", |job| job.module.len(), |app, n| app.log.info(format!("next {n}")));
        until_done(&mut app);
        assert!(logged(&app, Level::Info, "next 1"));
    }

    #[test]
    fn a_job_reads_the_module_as_it_was_and_its_edits_are_one_command() {
        let mut app = app("ui-job-edits");
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        // An unsaved change is in what the job reads.
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// changed".to_vec()) };
        app.apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        app.start_job(
            "Shout",
            move |job| {
                let text = job.module.get(&key).unwrap().to_ascii_uppercase();
                vec![mg_edit::Edit::SetResource { key, data: Some(text) }]
            },
            |app, edits| app.apply(mg_edit::Command::new("Shout", edits)).unwrap(),
        );
        until_done(&mut app);
        let ws = app.ws.as_mut().unwrap();
        assert_eq!(ws.module.get(&key), Some(&b"// CHANGED"[..]));
        assert_eq!(ws.can_undo(), Some("Shout"));
        ws.undo().unwrap();
        assert_eq!(ws.module.get(&key), Some(&b"// changed"[..]));
    }

    /// In the window: the job's name and Cancel, and nothing else takes
    /// input until it is over.
    #[test]
    fn the_window_shows_the_job_and_takes_nothing_else() {
        use egui_kittest::Harness;
        use egui_kittest::kittest::Queryable;
        let mut app = app("ui-job-window");
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// changed".to_vec()) };
        app.apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        let (go, wait) = mpsc::channel::<()>();
        app.start_job(
            "Count Resources",
            move |job| {
                job.progress.step(3, 12);
                job.progress.say("hello.nss");
                wait.recv().ok();
                job.module.len()
            },
            |app, n| app.log.info(format!("counted {n}")),
        );
        let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        let progress = h.state().job.as_ref().unwrap().progress.clone();
        while progress.note().is_empty() {
            std::thread::yield_now();
        }
        h.run_steps(2);
        h.get_by_label("Count Resources");
        h.get_by_label("hello.nss");
        h.get_by_label("3 of 12");
        // (A look at it: `target/test-output/ui-job-window-look/job.png`.)
        mg_testkit::gpu::hold();
        if let Ok(img) = h.render() {
            let _ = img.save(mg_testkit::scratch_dir("ui-job-window-look").join("job.png"));
        }
        mg_testkit::gpu::release();
        // Undo's key does nothing now.
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("edit"));
        // Cancel: it says so until the work stops, then nothing is handed over.
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        h.get_by_label("Canceling…");
        go.send(()).unwrap();
        let started = Instant::now();
        while h.state().busy() {
            h.run_steps(1);
            std::thread::sleep(Duration::from_millis(5));
            assert!(started.elapsed() < Duration::from_secs(20), "the job never ended");
        }
        h.run_steps(2);
        assert!(h.query_by_label("Count Resources").is_none(), "the window is gone");
        assert!(logged(h.state(), Level::Info, "Count Resources: canceled"));
        assert!(!logged(h.state(), Level::Info, "counted"));
        // And the keys work again.
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), None);
    }

    #[test]
    fn without_a_window_to_keep_alive_a_job_is_done_when_it_starts() {
        let mut app = app("ui-job-inline");
        app.background_jobs = false;
        app.start_job(
            "Count",
            |job| job.module.len(),
            |app, n| app.log.info(format!("counted {n}")),
        );
        assert!(!app.busy() && logged(&app, Level::Info, "counted 1"));
        // No module: no job.
        app.close();
        app.start_job("Count", |job| job.module.len(), |app, n| app.log.info(format!("{n}")));
        assert!(!app.busy() && logged(&app, Level::Error, "Count needs an open module"));
    }
}
