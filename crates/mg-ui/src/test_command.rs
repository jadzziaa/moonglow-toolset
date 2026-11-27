//! Options › Test Module's command of the builder's own (GitHub issue 52):
//! what Test Module and Test From Here run in place of the game, such as a
//! persistent world's server script (NWNX, a database, WSL), and then,
//! where chosen, the game joining the server it started.
//!
//! The command runs by the system's shell in the module's folder
//! (`project_commands::shell`), `{module_path}` and `{module_name}` filled
//! in (`test_module::fill_in`), in the background: the window goes on
//! meanwhile, what the command prints comes into the log as it comes, and
//! how it ended is said. It runs until it ends, the next test (which stops
//! it first, so that a server of the old module lets go of the port), or
//! Moonglow closes (which stops it too: a test's server is not left behind).
//! With Then start the game, a thread asks the address for a server
//! (`mg_serve::answers`) for up to [`JOIN_WITHIN`] and the game is
//! started connected to it once one answers.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::Moonglow;

/// How long a test waits for the server its command starts to answer
/// before it says so and starts no game.
pub(crate) const JOIN_WITHIN: Duration = Duration::from_secs(120);

/// The game to start once the server answers: the client, its user
/// directory, the address and password, and whether through Steam.
pub(crate) struct Join {
    pub client: PathBuf,
    pub user: PathBuf,
    pub address: String,
    pub password: String,
    pub steam: bool,
}

/// What the command's threads hand the window: its lines (and whether on
/// standard error), and the wait for its server once it is over (whether
/// one answered).
#[derive(Default)]
struct Incoming {
    lines: Vec<(bool, String)>,
    answered: Option<bool>,
}

/// The test command running: the shell, what comes from it, and the game
/// to start once its server answers.
pub(crate) struct Running {
    child: Child,
    incoming: Arc<Mutex<Incoming>>,
    join: Option<Join>,
}

impl Drop for Running {
    /// Moonglow closing, or the next test: the command and what it
    /// started stopped.
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            crate::project_commands::stop(&mut self.child);
        }
    }
}

/// Reads a pipe a line at a time into `incoming` (bytes that aren't UTF-8
/// as the replacement character), until it closes.
fn read_lines(
    pipe: Option<impl std::io::Read + Send + 'static>,
    err: bool,
    incoming: &Arc<Mutex<Incoming>>,
) {
    let Some(pipe) = pipe else { return };
    let incoming = Arc::clone(incoming);
    std::thread::spawn(move || {
        use std::io::BufRead as _;
        let mut reader = std::io::BufReader::new(pipe);
        let mut line = Vec::new();
        while reader.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
            let text = String::from_utf8_lossy(&line).trim_end_matches(['\n', '\r']).to_string();
            incoming.lock().unwrap_or_else(|e| e.into_inner()).lines.push((err, text));
            line.clear();
        }
    });
}

/// The next test, of any kind (this command again, the game, Test
/// Module's local server): a command still running from the last test
/// stopped first, and what it started, so that its server lets go of the
/// port and of the module.
pub(crate) fn stop_last(app: &mut Moonglow) {
    if let Some(old) = app.test_command.take() {
        app.log.info("Test Module: the last test's command stopped");
        drop(old);
    }
}

/// Runs the test command `line` for the module at `module_path` (named
/// `module_name`), stopping one still running from the last test first;
/// `join`, the game started once a server answers.
pub(crate) fn start(
    app: &mut Moonglow,
    line: &str,
    module_path: &Path,
    module_name: &str,
    join: Option<Join>,
) {
    stop_last(app);
    let text = crate::test_module::fill_in(line, module_path, module_name);
    let folder = module_path.parent().filter(|p| p.is_dir()).unwrap_or(Path::new("."));
    let mut command = crate::project_commands::shell(&text, folder, false);
    command.env("MOONGLOW_MODULE", module_path);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            app.log.error(format!("Test Module: the command could not be run: {e}"));
            return;
        }
    };
    app.log.info(format!("Test Module: {text} (in {})", folder.display()));
    let incoming = Arc::new(Mutex::new(Incoming::default()));
    read_lines(child.stdout.take(), false, &incoming);
    read_lines(child.stderr.take(), true, &incoming);
    if let Some(join) = &join {
        app.log.info(format!("Test Module: waiting for the server at {}", join.address));
        let (address, incoming) = (join.address.clone(), Arc::clone(&incoming));
        std::thread::spawn(move || {
            let answered = mg_serve::answers(&address, JOIN_WITHIN);
            incoming.lock().unwrap_or_else(|e| e.into_inner()).answered = Some(answered);
        });
    }
    // (Options › General's Minimize Toolset on test module: now, or as the
    // game starts.)
    if join.is_none() {
        app.minimize_requested = app.settings.minimize_on_test;
    }
    app.test_command = Some(Running { child, incoming, join });
}

/// Each frame: the command's lines into the log, the game started once its
/// server answered (or said that none did), and how the command ended.
pub(crate) fn tick(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(running) = &mut app.test_command else { return };
    let ended = running.child.try_wait().ok().flatten();
    if ended.is_some() {
        // (Its last lines first: its pipes close as it ends.)
        std::thread::sleep(Duration::from_millis(50));
    }
    let (lines, answered) = {
        let mut i = running.incoming.lock().unwrap_or_else(|e| e.into_inner());
        (std::mem::take(&mut i.lines), i.answered.take())
    };
    let join = answered.and_then(|a| running.join.take().map(|j| (a, j)));
    for (err, line) in lines {
        if err {
            app.log.warn(format!("  {line}"));
        } else {
            app.log.info(format!("  {line}"));
        }
    }
    match join {
        Some((true, join)) => {
            let mut game = crate::test_module::join_command(
                &join.client,
                &join.user,
                &join.address,
                &join.password,
                join.steam,
            );
            match game.spawn() {
                Ok(game) => {
                    crate::test_module::let_game_run(game);
                    app.log.info(format!("Test Module: the game joins {}", join.address));
                    if let Some(line) = crate::test_module::steam_line(&join.client, join.steam) {
                        app.log.info(line);
                    }
                    app.minimize_requested = app.settings.minimize_on_test;
                }
                Err(e) => app.log.error(format!("Test Module: {}: {e}", join.client.display())),
            }
        }
        Some((false, join)) => app.log.error(format!(
            "Test Module: no server answered at {} within {} s: the game was not started",
            join.address,
            JOIN_WITHIN.as_secs()
        )),
        None => {}
    }
    if let Some(status) = ended {
        match status.code() {
            Some(0) => app.log.info("Test Module: the command ended"),
            Some(code) => {
                app.log.error(format!("Test Module: the command ended with exit code {code}"))
            }
            None => app.log.warn("Test Module: the command was stopped"),
        }
        app.test_command = None;
        return;
    }
    ctx.request_repaint_after(Duration::from_millis(250));
}
