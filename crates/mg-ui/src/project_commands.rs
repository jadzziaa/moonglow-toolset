//! File › Project: the commands a project names in its `moonglow.toml`
//! (`[commands]`), such as a team's update and submit scripts (The
//! Domains of Dread's builder proposal, C8), run from the window in place
//! of "close Moonglow, run the script".
//!
//! A command runs only when chosen, never when a module opens. Before it
//! runs, the module is saved (asked as on closing, and about others'
//! areas as any save is) and its files let go: the look at the project's
//! files changed outside stops, and the module's haks are closed, so that
//! a checkout may write over any of them (Windows refuses to replace a
//! file another program has open). The command runs in the project's
//! folder, by the system's shell (`sh -c` on Linux and macOS, `cmd /C` on
//! Windows), its output in the log as it comes, and its exit code said.
//! Then the haks are opened again and the project's files read again, as
//! for any change made outside Moonglow: what the command changed is in
//! the module, and the claims table and the project file are read anew.
//!
//! A command named by a file that came with a checkout is a program the
//! builder did not choose: the first time it is run, and whenever its
//! text changes, the builder is shown it and asked to allow it, as a
//! plugin's reach is asked for. What was allowed is kept in the builder's
//! own `.moonglow/user.toml` (`[allowed_commands]`), by name and text.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use mg_module::project_file::{ProjectCommand, ProjectFile};

use crate::Moonglow;

/// A command's output, from its thread to the window's: each line, and
/// whether it came on standard error.
type Output = Arc<Mutex<Vec<(bool, String)>>>;

/// The project's commands' state.
#[derive(Debug, Default)]
pub(crate) struct CommandsState {
    /// The command asked about (Allow and Run).
    pub(crate) ask: Option<Ask>,
    /// The running command's output not yet in the log.
    output: Output,
}

/// A command waiting to be allowed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ask {
    pub name: String,
    pub label: String,
    pub run: String,
    pub root: PathBuf,
}

/// The project's commands, with its root, where the module open is a
/// project's (a nasher project or a module folder) that names any.
pub(crate) fn commands(app: &Moonglow) -> Option<(PathBuf, Vec<ProjectCommand>)> {
    let ws = app.ws.as_ref()?;
    let root = ws.module.location.as_ref().and_then(ProjectFile::root_of)?.to_path_buf();
    let commands = ws.module.project_file.commands.clone();
    (!commands.is_empty()).then_some((root, commands))
}

/// Runs the project's command `name` (the module was saved, or the
/// builder said not to): asks first, where it was never allowed as it
/// reads now.
pub(crate) fn run(app: &mut Moonglow, name: &str) {
    let Some((root, commands)) = commands(app) else {
        app.log.warn("The project names no commands (moonglow.toml's [commands])");
        return;
    };
    let Some(c) = commands.into_iter().find(|c| c.name == name) else {
        app.log.error(format!("The project names no command {name} (moonglow.toml's [commands])"));
        return;
    };
    let Some(text) = c.run.clone() else {
        app.log.warn(format!(
            "{}: moonglow.toml's [commands] gives it no command for this system",
            c.label
        ));
        return;
    };
    let allowed =
        app.ws.as_ref().is_some_and(|ws| ws.module.project_file.command_allowed(&c.name, &text));
    if !allowed {
        app.project_commands.ask = Some(Ask { name: c.name, label: c.label, run: text, root });
        return;
    }
    start(app, &c.label, &text, &root);
}

/// Allows a command as it reads (in the builder's own file), and runs it.
fn allow_and_run(app: &mut Moonglow, ask: &Ask) {
    if let Err(e) = ProjectFile::set_allowed_command(&ask.root, &ask.name, Some(&ask.run)) {
        app.log.error(format!("{}: not allowed (.moonglow/user.toml: {e})", ask.label));
        return;
    }
    if let Some(ws) = app.ws.as_mut() {
        let had = ws.module.project_file.found;
        if ws.module.reread_project_file() {
            app.log_project_file(true, had);
        }
    }
    start(app, &ask.label, &ask.run, &ask.root);
}

/// The shell command that runs `text`, in `root`.
fn shell(text: &str, root: &Path) -> Command {
    #[cfg(windows)]
    let mut c = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        // (As typed: cmd reads its own quotes. No console window.)
        c.arg("/C").raw_arg(text).creation_flags(0x0800_0000);
        c
    };
    #[cfg(not(windows))]
    let mut c = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(text);
        c
    };
    c.current_dir(root).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    c
}

/// How a command ended.
#[derive(Debug)]
enum Ended {
    Exited(Option<i32>),
    Failed(String),
    Canceled,
}

/// Runs a command as a job (the window takes nothing else meanwhile),
/// the module's files let go, and reads them again after.
fn start(app: &mut Moonglow, label: &str, text: &str, root: &Path) {
    // (Another job under way: nothing is let go for one that can't start.)
    if let Some(job) = &app.job {
        app.log.warn(format!("{label}: waiting for {} to finish", job.title));
        return;
    }
    app.log.info(format!("{label}: {text} (in {})", root.display()));
    app.pause_outside();
    let released = app.release_haks();
    let output = app.project_commands.output.clone();
    let mut command = shell(text, root);
    command.env("MOONGLOW_PROJECT", root);
    if let Some(code) =
        app.ws.as_ref().and_then(|ws| ws.module.project_file.user.builder_code.clone())
    {
        command.env("MOONGLOW_BUILDER", code);
    }
    let label = label.to_string();
    let title = label.clone();
    let finished = std::rc::Rc::new(std::cell::Cell::new(false));
    let done = finished.clone();
    app.start_job(
        title,
        move |cx| {
            let mut child = match command.spawn() {
                Ok(c) => c,
                Err(e) => return Ended::Failed(e.to_string()),
            };
            // Each line into the log as it comes, the last in the job's
            // window.
            let read = |pipe: Option<Box<dyn std::io::Read + Send>>, err: bool| {
                let (output, progress) = (output.clone(), cx.progress.clone());
                std::thread::spawn(move || {
                    let Some(pipe) = pipe else { return };
                    for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                        progress.say(line.clone());
                        output.lock().unwrap_or_else(|e| e.into_inner()).push((err, line));
                    }
                })
            };
            let out = read(child.stdout.take().map(|p| Box::new(p) as _), false);
            let errs = read(child.stderr.take().map(|p| Box::new(p) as _), true);
            let ended = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Ended::Exited(status.code()),
                    Ok(None) if cx.progress.cancelled() => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break Ended::Canceled;
                    }
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                    Err(e) => break Ended::Failed(e.to_string()),
                }
            };
            // (A program it started may hold the pipes past its end.)
            if !matches!(ended, Ended::Canceled) {
                let _ = out.join();
                let _ = errs.join();
            }
            ended
        },
        move |app, ended| {
            done.set(true);
            take_output(app);
            match ended {
                Ended::Exited(Some(0)) => app.log.info(format!("{label}: done")),
                Ended::Exited(Some(code)) => {
                    app.log.error(format!("{label} failed: it ended with exit code {code}"))
                }
                Ended::Exited(None) => app.log.error(format!("{label} was stopped")),
                Ended::Failed(e) => app.log.error(format!("{label} could not be run: {e}")),
                Ended::Canceled => app.log.warn(format!("{label}: canceled (stopped)")),
            }
            reopened(app, released);
        },
    );
    // (A job that could not start: the files are taken back at once.)
    if !app.busy() && !finished.get() {
        reopened(app, released);
    }
}

/// After a command: the haks opened again, and the module's files read
/// again (on a thread of their own, as the watch's are).
fn reopened(app: &mut Moonglow, released: bool) {
    take_output(app);
    if released {
        app.sync_haks();
    }
    if app.background_jobs {
        app.look_outside();
    } else {
        app.reload_project_files();
    }
}

/// The output come since last time, into the log.
fn take_output(app: &mut Moonglow) {
    let lines =
        std::mem::take(&mut *app.project_commands.output.lock().unwrap_or_else(|e| e.into_inner()));
    for (err, line) in lines {
        if err {
            app.log.warn(format!("  {line}"));
        } else {
            app.log.info(format!("  {line}"));
        }
    }
}

/// What File › Project's entry for a command says on hover.
pub(crate) fn tip(c: &ProjectCommand) -> String {
    match &c.run {
        Some(run) => format!(
            "Saves the module, runs {run} in the project's folder, and reads its files again"
        ),
        None => "The project's moonglow.toml gives it no command for this system".to_string(),
    }
}

/// The question before a command runs that was never allowed as it
/// reads; and the running command's output, into the log.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    take_output(app);
    let Some(ask) = app.project_commands.ask.clone() else { return };
    let mut open = true;
    let (mut run, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Run a Project Command?", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(480.0);
            ui.label(format!(
                "The project's moonglow.toml asks to run {}, in {}:",
                ask.label,
                ask.root.display()
            ));
            ui.add_space(4.0);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&ask.run).monospace()).wrap());
            });
            ui.add_space(4.0);
            ui.label(
                "It runs as you, with everything you may do: a project's files can name any \
                 program. Allow it only if you trust where this project came from, and the \
                 command reads as you expect.",
            );
            ui.weak(
                "Allowed, it runs without asking until its text changes (kept in this \
                 project's .moonglow/user.toml). Moonglow saves the module first, and reads its \
                 files again after.",
            );
            ui.separator();
            ui.horizontal(|ui| {
                run = ui.button("Allow and Run").clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if run {
        app.project_commands.ask = None;
        allow_and_run(app, &ask);
    } else if cancel || !open {
        app.project_commands.ask = None;
    }
}
