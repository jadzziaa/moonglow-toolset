//! The game's headless server and the game client joined to it, in one
//! sandbox (`tools/nwclient/run-server-and-client.sh`): for readings of
//! what a client sees of a server (the DM client's Creator, an NWSync
//! download, a woman's character's talk table).
//!
//! The server is started through [`mg_serve::Server::start`] with its
//! defaults (`-publicserver 0`, passwords nobody was told), the sandbox
//! script standing in for its program: the script reads the port and the
//! password the client joins with from the server's own arguments. The
//! sandbox's network is a loopback of its own; the client draws on the
//! off-screen display (`DISPLAY`, never `:0`).
//!
//! **A CD key.** The sandboxed client has no Steam, so no CD key: its
//! Multiplayer asks for one ("Enter CD-KEY", seen 2026-10-29, 89.8193.37),
//! and `+connect` ends in "Your CD Key is unauthorized". A key is not made
//! up here: [`start`] takes the `cdkey.ini` that `MG_CLIENT_CDKEY` names
//! (the owner's to give; copied into the scratch user directory, never
//! sent anywhere but the sandbox's loopback) and is an error without it.
//! The server checks no key: its master server is out of reach in the
//! sandbox, so its `masterserver.key-authentication` is `if-reachable`
//! (with `always`, the default, nobody gets in without the master server).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// How the client joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// A player, with the player password.
    Player,
    /// The DM client (`-dmc`), with the DM password.
    Dm,
}

/// A server and its client in one sandbox; both end when it is dropped.
#[derive(Debug)]
pub struct Joined {
    pub server: mg_serve::Server,
    /// The scratch directory: the client's user directory is `user`, the
    /// server's `server`.
    pub scratch: PathBuf,
}

/// The repository's root.
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The server's user directory in `scratch`: where the module goes
/// (`modules/<name>.mod`) before [`start`].
pub fn server_dir(scratch: &Path) -> PathBuf {
    scratch.join("server")
}

/// The client's user directory in `scratch` (its `settings.tml`, its
/// `nwsync` folder after a download, its logs).
pub fn client_dir(scratch: &Path) -> PathBuf {
    scratch.join("user")
}

/// The CD key the client joins with: the file `MG_CLIENT_CDKEY` names.
pub fn cd_key() -> Option<PathBuf> {
    std::env::var_os("MG_CLIENT_CDKEY").map(PathBuf::from).filter(|p| p.is_file())
}

/// Starts the server on `<scratch>/server/modules/<module>.mod` and the
/// client joined to it as `join`; `web`, if given, is a folder served on
/// the sandbox's loopback over HTTP (an NWSync repository: the server is
/// given `-nwsyncurl` for it, and `server_args` would add `-nwsynchash`).
pub fn start(
    scratch: &Path,
    module: &str,
    join: Join,
    web: Option<&Path>,
    server_args: &[&str],
) -> io::Result<Joined> {
    let key = cd_key().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the sandboxed client has no CD key: MG_CLIENT_CDKEY names none (a cdkey.ini)",
        )
    })?;
    let root = crate::nwn_root()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no game install (NWN_ROOT)"))?;
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":1".into());
    if display == ":0" {
        return Err(io::Error::other("refusing the desktop display :0"));
    }
    let user = client_dir(scratch);
    std::fs::create_dir_all(&user)?;
    std::fs::copy(&key, user.join("cdkey.ini"))?;
    let server = server_dir(scratch);
    std::fs::create_dir_all(&server)?;
    std::fs::write(
        server.join("settings.tml"),
        "[masterserver]\n\t[masterserver.key-authentication]\n\t\tmode = \"if-reachable\"\n",
    )?;
    let mut env: Vec<(OsString, OsString)> = vec![
        ("MG_SCRATCH".into(), scratch.into()),
        ("MG_CLIENT".into(), if join == Join::Dm { "dm" } else { "player" }.into()),
        ("NWN_ROOT".into(), root.into()),
        ("DISPLAY".into(), display.into()),
    ];
    if let Some(web) = web {
        env.push(("MG_WEB".into(), web.into()));
        // (The client ends itself after an NWSync download, and its own
        // restart doesn't come back in the sandbox: started once more.)
        env.push(("MG_CLIENT_RUNS".into(), "3".into()));
    }
    let server = mg_serve::Server::start(&mg_serve::Options {
        server: repo().join("tools/nwclient/run-server-and-client.sh"),
        user_dir: server,
        module: module.into(),
        name: "moonglow-test".into(),
        env,
        args: server_args.iter().map(Into::into).collect(),
        ..Default::default()
    })?;
    Ok(Joined { server, scratch: scratch.to_path_buf() })
}

impl Joined {
    /// The client's log (the game's engine log of the client).
    pub fn client_log(&self) -> String {
        std::fs::read_to_string(client_dir(&self.scratch).join("logs/nwengineLog.txt"))
            .unwrap_or_default()
    }

    /// Waits until the server's log has a line containing `line`, or
    /// `patience` passes; whether it came.
    pub fn wait_for(&self, line: &str, patience: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < patience {
            if self.server.log().contains(line) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        false
    }

    /// The client's window as `<scratch>/<name>.png`; whether it was taken.
    pub fn winshot(&self, name: &str) -> Option<PathBuf> {
        let shot = self.scratch.join(format!("{name}.png"));
        Command::new("python3")
            .arg(repo().join("tools/aurora/xdrive.py"))
            .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
            .arg(&shot)
            .status()
            .is_ok_and(|s| s.success())
            .then_some(shot)
    }

    /// Ends the sandbox (the server and the client with it).
    pub fn stop(self) {
        self.server.stop(Duration::from_secs(1));
    }
}
