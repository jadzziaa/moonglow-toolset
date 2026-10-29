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
//! file another program has open); a command that writes none of the
//! module's files (`writes = false`, a submit) keeps them open, and the
//! open areas aren't drawn again after. The command runs in the project's
//! folder, by the system's shell (`sh -c` on Linux and macOS, `cmd /C` on
//! Windows), its output in the log as it comes, and its exit code said;
//! Cancel stops the shell and what it started (its process group, or
//! tree of processes on Windows). Then the haks are opened again and the
//! project's files read again, as for any change made outside Moonglow:
//! what the command changed is in the module, and the claims table and
//! the project file are read anew.
//!
//! A command is in File › Project, the Command Palette and Options ›
//! Keyboard (the keys it asks for, `key`, until the builder chooses
//! others), and runs the same way from each.
//!
//! A command named by a file that came with a checkout is a program the
//! builder did not choose: the first time it is run, and whenever its
//! text changes, the builder is shown it and asked to allow it, as a
//! plugin's reach is asked for. What was allowed is kept in Moonglow's
//! own settings (`allowed_commands`), by the project's canonical path,
//! the command's name and its exact text: never in the project's files,
//! which a repository could carry (a `[allowed_commands]` found there is
//! said and not read).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use mg_module::project_file::{ProjectCommand, ProjectFile};

/// A project's folder as its allowed commands are kept: its canonical
/// path (the same folder reached another way is the same project).
fn project_key(root: &Path) -> String {
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()).display().to_string()
}

/// Whether the builder allowed a project's command, as it reads now.
fn allowed(app: &Moonglow, root: &Path, name: &str, run: &str) -> bool {
    let project = project_key(root);
    (app.settings.allowed_commands.iter()).any(|(p, n, t)| *p == project && n == name && t == run)
}

use egui::Ui;

use crate::Moonglow;

/// A command's output as it comes, from its threads to the window's:
/// each line, and whether it came on standard error; and the line each
/// has begun and not ended yet (a question, "Password: ").
#[derive(Debug, Default)]
struct Incoming {
    lines: Vec<(Said, String)>,
    partial: [Vec<u8>; 2],
}

impl Incoming {
    /// The lines begun, as text.
    fn begun(&self) -> [String; 2] {
        self.partial.each_ref().map(|p| line_text(p))
    }

    /// The lines begun ended (an answer to them typed, as a terminal
    /// ends the line when Enter is pressed).
    fn end_lines(&mut self) {
        for (said, p) in [Said::Out, Said::Error].into_iter().zip(&mut self.partial) {
            if !p.is_empty() {
                self.lines.push((said, line_text(&std::mem::take(p))));
            }
        }
    }
}

type Output = Arc<Mutex<Incoming>>;

/// The most lines of a command's output its window keeps (the log has
/// them all).
const KEPT: usize = 20_000;

/// The project's commands' state.
#[derive(Debug, Default)]
pub(crate) struct CommandsState {
    /// The command asked about (Allow and Run).
    pub(crate) ask: Option<Ask>,
    /// The running command's output not yet in the log.
    output: Output,
    /// The last command run: its output whole, for its window.
    last: Option<Last>,
    /// The running command's input, where it reads what is typed.
    input: Option<Input>,
    /// Whether a command is running (the job's window shows its output),
    /// whether the project's files were let go for it (a project's
    /// command; not an upload), and the module's haks.
    running: bool,
    let_go: bool,
    released: bool,
    /// The Command Output window is open.
    pub(crate) show: bool,
    /// Lines in the command's window not yet in the log (taken as an
    /// answer was typed).
    unlogged: Vec<(Said, String)>,
}

/// What a line of a command's output is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Said {
    Out,
    Error,
    /// What the builder typed for it (a hidden line is not kept).
    Typed,
    /// Moonglow's own: which program runs now, of several.
    Note,
}

/// The last command run, as its window shows it.
#[derive(Debug, Default)]
struct Last {
    label: String,
    lines: std::collections::VecDeque<(Said, String)>,
    /// How many of its first lines are in the log only.
    dropped: usize,
    /// How it ended; none while it runs.
    ended: Option<String>,
}

impl Last {
    fn push(&mut self, said: Said, line: String) {
        if self.lines.len() == KEPT {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back((said, line));
    }
}

/// A running command's input line.
#[derive(Debug)]
struct Input {
    text: String,
    hidden: bool,
    /// A line to its standard input; `None` closes it.
    send: std::sync::mpsc::Sender<Option<String>>,
    /// The field has been given the keys.
    focused: bool,
}

/// A command waiting to be allowed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ask {
    pub name: String,
    pub label: String,
    pub run: String,
    pub root: PathBuf,
    pub writes: bool,
    pub input: bool,
    /// The project's NWSync upload (`[nwsync] upload`), not one of its
    /// `[commands]`.
    pub upload: Option<UploadAsk>,
}

/// The project's NWSync upload, waiting to be allowed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UploadAsk {
    /// The repository it sends, and its parts, in turn.
    pub folder: PathBuf,
    pub parts: Vec<mg_module::nwsync::Part>,
    /// From the builder's own file (`.moonglow/user.toml`).
    pub own: bool,
}

/// The name a project's NWSync upload is allowed by, among its commands':
/// none of theirs is so named ([`ProjectFile`]'s names are letters,
/// digits, `_` and `-`).
pub(crate) const UPLOAD_NAME: &str = "[nwsync] upload";

/// The id a project's command's keys are kept under (Options ›
/// Keyboard): by its name, so that a team's `update` keeps its key from
/// one project to the next. (A plugin's id has no `:`.)
pub(crate) fn key_id(name: &str) -> String {
    format!("project:{name}")
}

/// The keys the open project's commands ask for (`key`), as their own
/// until the builder chooses others; and those of each, with its name in
/// Options › Keyboard.
pub(crate) fn register_keys(keymap: &mut crate::keys::Keymap, commands: &[ProjectCommand]) {
    for c in commands {
        let keys = c.key.as_deref().and_then(crate::keys::from_text).into_iter().collect();
        keymap.set_default(&key_id(&c.name), keys);
    }
}

/// The open project's commands as Options › Keyboard lists them: the id
/// of each one's keys, and its name.
pub(crate) fn key_rows(app: &Moonglow) -> Vec<(String, String)> {
    let commands = commands(app).map(|(_, c)| c).unwrap_or_default();
    commands.into_iter().map(|c| (key_id(&c.name), c.label)).collect()
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
    if !allowed(app, &root, &c.name, &text) {
        let (writes, input) = (c.writes, c.input);
        let ask =
            Ask { name: c.name, label: c.label, run: text, root, writes, input, upload: None };
        app.project_commands.ask = Some(ask);
        return;
    }
    start(app, &c.label, &text, &root, (c.writes, c.input));
}

/// Whether the project's NWSync upload, as it reads now, was allowed.
pub(crate) fn upload_allowed(app: &Moonglow, root: &Path, run: &str) -> bool {
    allowed(app, root, UPLOAD_NAME, run)
}

/// Allows a command as it reads (in Moonglow's settings, for this
/// project), and runs it.
fn allow_and_run(app: &mut Moonglow, ask: &Ask) {
    let project = project_key(&ask.root);
    // (One text a command: the one allowed before is forgotten.)
    app.settings.allowed_commands.retain(|(p, n, _)| !(*p == project && *n == ask.name));
    app.settings.allowed_commands.push((project, ask.name.clone(), ask.run.clone()));
    match &ask.upload {
        Some(u) => crate::nwsync_view::upload(app, &u.folder, &u.parts),
        None => start(app, &ask.label, &ask.run, &ask.root, (ask.writes, ask.input)),
    }
}

/// The shell command that runs `text`, in `root`, its output read and
/// its input closed (`input`: read from Moonglow).
pub(crate) fn shell(text: &str, root: &Path, input: bool) -> Command {
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
        use std::os::unix::process::CommandExt;
        let mut c = Command::new("sh");
        // (A process group of its own, the shell its leader: Cancel stops
        // what the script started with it.)
        c.arg("-c").arg(text).process_group(0);
        c
    };
    let stdin = if input { Stdio::piped() } else { Stdio::null() };
    c.current_dir(root).stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::piped());
    // (git asks for a password on the terminal Moonglow was started from,
    // which a command in a process group of its own can't read from: it
    // waited there until canceled. So it says it can't ask, unless the
    // builder chose otherwise; a password program, `GIT_ASKPASS` or a
    // credential helper is asked as before.)
    if std::env::var_os("GIT_TERMINAL_PROMPT").is_none() {
        c.env("GIT_TERMINAL_PROMPT", "0");
    }
    c
}

/// A pipe's bytes as text, a line each (its line end taken off): as
/// UTF-8, a byte that isn't read as the replacement character.
fn line_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.trim_end_matches(['\n', '\r']).to_string()
}

/// Reads a pipe into `output` as it comes: whole lines, and the line
/// begun (a question waiting for its answer) as it stands.
fn read_pipe(pipe: Option<impl std::io::Read>, err: bool, output: &Output) {
    let Some(mut pipe) = pipe else { return };
    let mut buf = [0u8; 4096];
    let at = usize::from(err);
    let said = if err { Said::Error } else { Said::Out };
    loop {
        let n = match pipe.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut o = output.lock().unwrap_or_else(|e| e.into_inner());
        o.partial[at].extend_from_slice(&buf[..n]);
        while let Some(end) = o.partial[at].iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = o.partial[at].drain(..=end).collect();
            o.lines.push((said, line_text(&line)));
        }
    }
    let mut o = output.lock().unwrap_or_else(|e| e.into_inner());
    let rest = std::mem::take(&mut o.partial[at]);
    if !rest.is_empty() {
        o.lines.push((said, line_text(&rest)));
    }
}

/// Stops a command and what it started: on Linux and macOS its shell's
/// process group ([`shell`] makes the shell its leader), asked to end
/// (TERM: a `git` takes its lock files off) and, still there after two
/// seconds, made to (KILL); on Windows the shell's tree of processes
/// (`taskkill /T /F`). Then the shell is waited for.
fn stop(child: &mut Child) {
    #[cfg(unix)]
    {
        use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};
        use std::time::{Duration, Instant};
        if let Some(group) = i32::try_from(child.id()).ok().and_then(Pid::from_raw) {
            let _ = kill_process_group(group, Signal::TERM);
            let until = Instant::now() + Duration::from_secs(2);
            // (The shell is waited for meanwhile: a group is there while
            // a process of it is, its leader's end not yet waited for
            // too.)
            while test_kill_process_group(group).is_ok() && Instant::now() < until {
                let _ = child.try_wait();
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = kill_process_group(group, Signal::KILL);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // (Before the shell ends: the tree is found from it.)
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// How a command ended.
#[derive(Debug)]
pub(crate) enum Ended {
    Exited(Option<i32>),
    Failed(String),
    Canceled,
}

impl Ended {
    pub(crate) fn succeeded(&self) -> bool {
        matches!(self, Ended::Exited(Some(0)))
    }
}

/// One program of a run: the line Moonglow says before it (which of
/// several runs now), and the command.
pub(crate) struct Step {
    pub note: Option<String>,
    pub command: Command,
}

/// Programs run in turn as a job, their output in the job's window, the
/// log and Command Output: a project's command, or an NWSync upload.
pub(crate) struct Run {
    /// The job's name, and its output's.
    pub label: String,
    pub steps: Vec<Step>,
    /// They read what is typed (`input = true`).
    pub input: bool,
    /// Where all it said is shown afterwards, for the log's line of a
    /// failure.
    pub output_at: &'static str,
    /// The project's files let go while it runs and read again after (a
    /// project's command), and the haks too (one that `writes`); none for
    /// an upload, which touches none of them.
    pub let_go: Option<bool>,
}

/// Runs a project's command as a job (the window takes nothing else
/// meanwhile), the module's files let go (its haks only where it
/// `writes`), and reads them again after.
fn start(app: &mut Moonglow, label: &str, text: &str, root: &Path, (writes, input): (bool, bool)) {
    let mut command = shell(text, root, input);
    command.env("MOONGLOW_PROJECT", root);
    if let Some(code) =
        app.ws.as_ref().and_then(|ws| ws.module.project_file.user.builder_code.clone())
    {
        command.env("MOONGLOW_BUILDER", code);
    }
    // (Another job under way: said, and nothing let go.)
    if app.job.is_none() {
        app.log.info(format!("{label}: {text} (in {})", root.display()));
    }
    let run = Run {
        label: label.to_string(),
        steps: vec![Step { note: None, command }],
        input,
        output_at: "File › Project › Command Output",
        let_go: Some(writes),
    };
    run_steps(app, run, |_, _, _| {});
}

/// Runs `run`'s programs in turn as a job, each after the one before
/// succeeded; then `then` hears how the last one run ended, and which it
/// was (of the steps).
pub(crate) fn run_steps(
    app: &mut Moonglow,
    run: Run,
    then: impl FnOnce(&mut Moonglow, &Ended, usize) + 'static,
) {
    let Run { label, steps, input, output_at, let_go } = run;
    // (Another job under way: nothing is let go for one that can't start.)
    if let Some(job) = &app.job {
        app.log.warn(format!("{label}: waiting for {} to finish", job.title));
        return;
    }
    if let_go.is_some() {
        app.pause_outside();
    }
    // (A command that writes none of the module's files has its haks
    // left open: nothing to draw again after.)
    let released = let_go == Some(true) && app.release_haks();
    let output = app.project_commands.output.clone();
    *output.lock().unwrap_or_else(|e| e.into_inner()) = Incoming::default();
    let state = &mut app.project_commands;
    state.last = Some(Last { label: label.clone(), ..Last::default() });
    state.running = true;
    state.let_go = let_go.is_some();
    state.released = released;
    let (send, typed) = std::sync::mpsc::channel::<Option<String>>();
    state.input = input.then(|| Input { text: String::new(), hidden: false, send, focused: false });
    let title = label.clone();
    let finished = std::rc::Rc::new(std::cell::Cell::new(false));
    let done = finished.clone();
    let let_go = let_go.is_some();
    app.start_job_anywhere(
        title,
        move |cx| {
            let n = steps.len();
            // (End Input closes the input of what runs after too.)
            let mut closed = false;
            for (i, step) in steps.into_iter().enumerate() {
                if let Some(note) = step.note {
                    let mut o = output.lock().unwrap_or_else(|e| e.into_inner());
                    o.lines.push((Said::Note, note));
                }
                let ended = run_child(step.command, &output, &typed, &mut closed, &cx.progress);
                if !ended.succeeded() || i + 1 == n {
                    return (ended, i);
                }
            }
            (Ended::Exited(Some(0)), 0)
        },
        move |app, (ended, step): (Ended, usize)| {
            done.set(true);
            take_output(app);
            let said = match &ended {
                Ended::Exited(Some(0)) => {
                    app.log.info(format!("{label}: done"));
                    "Done.".to_string()
                }
                Ended::Exited(Some(code)) => {
                    app.log.error(format!(
                        "{label} failed: it ended with exit code {code} (all it said: {output_at})"
                    ));
                    format!("Failed: it ended with exit code {code}.")
                }
                Ended::Exited(None) => {
                    app.log.error(format!("{label} was stopped"));
                    "Stopped.".to_string()
                }
                Ended::Failed(e) => {
                    app.log.error(format!("{label} could not be run: {e}"));
                    format!("Could not be run: {e}")
                }
                Ended::Canceled => {
                    app.log.warn(format!("{label}: canceled (stopped)"));
                    "Canceled (stopped).".to_string()
                }
            };
            ended_with(app, said);
            if let_go {
                reopened(app, released);
            }
            then(app, &ended, step);
        },
    );
    // (A job that could not start, or called off: the files are taken
    // back at once.)
    if !app.busy() && !finished.get() {
        ended_with(app, "Canceled (stopped).".to_string());
        if let_go {
            reopened(app, released);
        }
    }
}

/// Runs one program to its end (or until the job is called off): its
/// output into `output` as it comes, and what is typed to its input.
fn run_child(
    mut command: Command,
    output: &Output,
    typed: &std::sync::mpsc::Receiver<Option<String>>,
    closed: &mut bool,
    progress: &crate::jobs::Progress,
) -> Ended {
    use std::io::Write;
    if *closed {
        command.stdin(Stdio::null());
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => return Ended::Failed(e.to_string()),
    };
    // Each line into the log as it comes, and the job's window.
    let read = |pipe: Option<Box<dyn std::io::Read + Send>>, err: bool| {
        let output = output.clone();
        std::thread::spawn(move || read_pipe(pipe, err, &output))
    };
    let out = read(child.stdout.take().map(|p| Box::new(p) as _), false);
    let errs = read(child.stderr.take().map(|p| Box::new(p) as _), true);
    let mut stdin = child.stdin.take();
    let ended = loop {
        // What is typed, to its input as it comes (closed with End
        // Input, or as the program ends).
        while let Ok(line) = typed.try_recv() {
            match (line, &mut stdin) {
                (Some(line), Some(pipe)) => {
                    let sent = pipe.write_all(format!("{line}\n").as_bytes());
                    if sent.and_then(|()| pipe.flush()).is_err() {
                        stdin = None;
                    }
                }
                (Some(_), None) => {}
                (None, _) => {
                    stdin = None;
                    *closed = true;
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ended::Exited(status.code()),
            Ok(None) if progress.cancelled() => {
                stop(&mut child);
                break Ended::Canceled;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => break Ended::Failed(e.to_string()),
        }
    };
    drop(stdin);
    // (A program it started may hold the pipes past its end.)
    if !matches!(ended, Ended::Canceled) {
        let _ = out.join();
        let _ = errs.join();
    }
    ended
}

/// The command over: how it ended, in its window; its input closed.
fn ended_with(app: &mut Moonglow, said: String) {
    let state = &mut app.project_commands;
    state.running = false;
    state.input = None;
    if let Some(last) = &mut state.last {
        last.ended = Some(said);
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

/// The output come since last time, into the log and the command's
/// window.
fn take_output(app: &mut Moonglow) {
    for (said, line) in take_output_into(&mut app.project_commands) {
        match said {
            Said::Error => app.log.warn(format!("  {line}")),
            Said::Note => app.log.info(line),
            Said::Out | Said::Typed => app.log.info(format!("  {line}")),
        }
    }
}

/// [`take_output`] into the command's window; returns the lines for the
/// log.
fn take_output_into(state: &mut CommandsState) -> Vec<(Said, String)> {
    let lines = std::mem::take(&mut state.output.lock().unwrap_or_else(|e| e.into_inner()).lines);
    if let Some(last) = &mut state.last {
        for (said, line) in &lines {
            last.push(*said, line.clone());
        }
    }
    let mut kept = std::mem::take(&mut state.unlogged);
    kept.extend(lines);
    kept
}

/// Whether a project's command is the job running (its window shows the
/// output, [`job_ui`]).
pub(crate) fn running(app: &Moonglow) -> bool {
    app.project_commands.running && app.busy()
}

/// The lines of a command's output, in a list that keeps to its end as
/// they come: its errors in the warning color, what was typed for it
/// grayed; and the lines begun and not ended (a question).
fn output_lines(ui: &mut Ui, last: &Last, partial: &[String; 2], height: f32, id: &str) {
    let row = ui.text_style_height(&egui::TextStyle::Monospace);
    let warn = ui.visuals().warn_fg_color;
    let pending: Vec<(Said, &String)> = [(Said::Out, &partial[0]), (Said::Error, &partial[1])]
        .into_iter()
        .filter(|(_, p)| !p.is_empty())
        .collect();
    let n = last.lines.len() + pending.len();
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(height)
        .auto_shrink([false, true])
        .stick_to_bottom(true)
        .show_rows(ui, row, n, |ui, rows| {
            for i in rows {
                let (said, text) = match last.lines.get(i) {
                    Some((said, text)) => (*said, text.as_str()),
                    None => {
                        let (said, text) = pending[i - last.lines.len()];
                        (said, text.as_str())
                    }
                };
                let text = egui::RichText::new(text).monospace();
                let text = match said {
                    Said::Out => text,
                    Said::Error => text.color(warn),
                    Said::Typed => text.weak(),
                    Said::Note => text.weak().italics(),
                };
                ui.add(egui::Label::new(text).extend());
            }
        });
}

/// In the job's window while a project's command runs: its output as it
/// comes, and for a command that reads what is typed (`input = true`), the
/// line to type it in.
pub(crate) fn job_ui(app: &mut Moonglow, ui: &mut Ui) {
    let output = app.project_commands.output.clone();
    let partial = output.lock().unwrap_or_else(|e| e.into_inner()).begun();
    let state = &mut app.project_commands;
    let Some(last) = &state.last else { return };
    egui::Frame::group(ui.style()).show(ui, |ui| {
        output_lines(ui, last, &partial, 260.0, "command-output-running");
    });
    let Some(input) = &mut state.input else { return };
    let mut sent = None;
    let mut close = false;
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut input.text)
            .password(input.hidden)
            .desired_width(300.0)
            .hint_text("what the command asks for");
        let r = ui.add(field);
        if !input.focused {
            r.request_focus();
            input.focused = true;
        }
        let entered = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let send =
            ui.button("Send").on_hover_text("Gives the command this line, as typed in a terminal");
        if send.clicked() || entered {
            sent = Some(std::mem::take(&mut input.text));
            r.request_focus();
        }
        let hide = ui.checkbox(&mut input.hidden, "Hide").on_hover_text(
            "Shows dots for what is typed, and keeps it out of the output (a password)",
        );
        // (The keys back in the line, to type in it.)
        if hide.changed() {
            r.request_focus();
        }
        close = ui
            .button("End Input")
            .on_hover_text("Closes the command's input: a command reading to its end goes on")
            .clicked();
    });
    if let Some(line) = sent {
        // Its question's line ended, as Enter ends it in a terminal, and
        // what was typed shown after it: before the line goes to the
        // command, whose next question would else be read onto the end of
        // this one ("Name: Secret: ") as it answers at once.
        let shown = if input.hidden { "(hidden)".to_string() } else { line.clone() };
        let send = input.send.clone();
        output.lock().unwrap_or_else(|e| e.into_inner()).end_lines();
        state.unlogged = take_output_into(state);
        if let Some(last) = &mut state.last {
            last.push(Said::Typed, format!("> {shown}"));
        }
        let _ = send.send(Some(line));
    }
    if close && let Some(input) = state.input.take() {
        let _ = input.send.send(None);
    }
}

/// File › Project › Command Output: the last command's output whole, how
/// it ended, and Copy.
fn output_window(app: &mut Moonglow, ctx: &egui::Context) {
    let output = app.project_commands.output.clone();
    let state = &mut app.project_commands;
    let Some(last) = state.last.as_ref().filter(|_| state.show) else { return };
    let mut open = true;
    let (mut copy, mut close) = (false, false);
    let title = "Command Output";
    let partial = output.lock().unwrap_or_else(|e| e.into_inner()).begun();
    crate::widgets::dialog(ctx, title, &mut open).default_size([640.0, 420.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            crate::widgets::section_heading(ui, &last.label);
            match &last.ended {
                Some(said) => ui.label(said),
                None => ui.weak("Running…"),
            };
        });
        if last.dropped > 0 {
            ui.weak(format!("({} lines before these are in the log only)", last.dropped));
        }
        let height = (ui.available_height() - 40.0).max(120.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            output_lines(ui, last, &partial, height, "command-output");
        });
        ui.horizontal(|ui| {
            copy = ui.button("Copy").on_hover_text("Copies the output, a line each").clicked();
            close = ui.button("Close").clicked();
        });
    });
    if copy {
        let text: Vec<&str> = last.lines.iter().map(|(_, l)| l.as_str()).collect();
        ctx.copy_text(text.join("\n"));
    }
    if !open || close {
        state.show = false;
    }
}

/// How the last run ended, where it was the one named `label`: `None`
/// inside while it runs; none where the last run was another.
pub(crate) fn last_of(app: &Moonglow, label: &str) -> Option<Option<String>> {
    let last = app.project_commands.last.as_ref().filter(|l| l.label == label)?;
    Some(last.ended.clone())
}

/// Whether there is a command's output to show (Command Output).
pub(crate) fn has_output(app: &Moonglow) -> bool {
    app.project_commands.last.is_some()
}

/// What File › Project's entry for a command says on hover.
pub(crate) fn tip(c: &ProjectCommand) -> String {
    match &c.run {
        Some(run) if !c.writes => format!(
            "Saves the module, runs {run} in the project's folder (the haks kept open: the \
             project says it writes none of the module's files), and reads its files again"
        ),
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
    // (A command canceled: its job hands nothing over, so what it let go
    // is taken back here. The haks stayed closed and the project's files
    // unwatched after a Cancel, before.)
    if app.project_commands.running && !app.busy() {
        ended_with(app, "Canceled (stopped).".to_string());
        let released = app.project_commands.released;
        if app.project_commands.let_go {
            reopened(app, released);
        }
    }
    output_window(app, ctx);
    let Some(ask) = app.project_commands.ask.clone() else { return };
    let mut open = true;
    let (mut run, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Run a Project Command?", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(480.0);
            let at = ask.upload.as_ref().map_or(&ask.root, |u| &u.folder);
            let file = match &ask.upload {
                Some(u) if u.own => ".moonglow/user.toml",
                _ => "moonglow.toml",
            };
            ui.label(format!(
                "The project's {file} asks to run {}, in {}:",
                ask.label,
                at.display()
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
            ui.weak(if ask.upload.is_some() {
                "Allowed, it runs without asking until its text changes (kept in Moonglow's own \
                 settings for this folder, not in the project). It runs once for each part of \
                 the repository in turn, so that the web server never serves a manifest whose \
                 data isn't there yet."
            } else {
                "Allowed, it runs without asking until its text changes (kept in Moonglow's own \
                 settings for this folder, not in the project). Moonglow saves the module first, \
                 and reads its files again after."
            });
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::time::{Duration, Instant};

    /// Cancel stops what the shell started, not the shell alone: a
    /// program it left running (a `git`) is stopped with it.
    #[test]
    fn stopping_a_command_stops_what_its_shell_started() {
        use rustix::process::{Pid, test_kill_process};
        let mut child = shell("sleep 60 & echo $!; wait", Path::new("."), false).spawn().unwrap();
        let mut line = String::new();
        let mut out = std::io::BufReader::new(child.stdout.take().unwrap());
        out.read_line(&mut line).unwrap();
        let sleeper = Pid::from_raw(line.trim().parse().unwrap()).unwrap();
        assert!(test_kill_process(sleeper).is_ok(), "running");
        let began = Instant::now();
        stop(&mut child);
        assert!(child.try_wait().unwrap().is_some(), "the shell ended");
        // (Ended, it is waited for by whoever took it on: a moment.)
        let until = Instant::now() + Duration::from_secs(10);
        while test_kill_process(sleeper).is_ok() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(test_kill_process(sleeper).is_err(), "what it started ended with it");
        assert!(began.elapsed() < Duration::from_secs(2), "asked, it ended at once");
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::time::{Duration, Instant};

    /// What the shell starts, for the test below: this test binary again,
    /// saying its process id and waiting.
    #[test]
    #[ignore = "started by stopping_a_command_stops_what_its_shell_started"]
    fn a_program_the_shell_started() {
        println!("started {}", std::process::id());
        std::thread::sleep(Duration::from_secs(60));
    }

    /// Cancel stops what the shell started, not the shell alone: a
    /// program `cmd` left running (a `git`) is stopped with it.
    #[test]
    fn stopping_a_command_stops_what_its_shell_started() {
        let exe = std::env::current_exe().unwrap();
        let text = format!(
            "\"{}\" --ignored --exact --nocapture \
             project_commands::tests::a_program_the_shell_started",
            exe.display()
        );
        let mut child = shell(&text, Path::new("."), false).spawn().unwrap();
        let out = std::io::BufReader::new(child.stdout.take().unwrap());
        let started = out
            .lines()
            .map_while(Result::ok)
            .find_map(|l| l.strip_prefix("started ").and_then(|p| p.trim().parse::<u32>().ok()));
        let started = started.expect("the program said its id");
        let running = |pid: u32| {
            let list = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                .output()
                .unwrap();
            String::from_utf8_lossy(&list.stdout).contains(&pid.to_string())
        };
        assert!(running(started), "running");
        stop(&mut child);
        assert!(child.try_wait().unwrap().is_some(), "the shell ended");
        let until = Instant::now() + Duration::from_secs(10);
        while running(started) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(!running(started), "what it started ended with it");
    }
}
