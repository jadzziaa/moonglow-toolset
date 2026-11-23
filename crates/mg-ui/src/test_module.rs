//! Build › Test Module (F9), as Aurora's: saves the module, then starts the
//! game on it (`nwmain -userdirectory <dir> +TestNewModule <module>`): the
//! first local-vault character, at the start location. The game finds the
//! module by name in the user directory's `modules` folder, so the module
//! must be saved there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The game client of the install at `root`, for this platform.
pub fn client_binary(root: &Path) -> Option<PathBuf> {
    let rel = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "bin/linux-arm64/nwmain-linux"
        } else {
            "bin/linux-x86/nwmain-linux"
        }
    } else if cfg!(target_os = "macos") {
        "bin/macos/nwmain.app/Contents/MacOS/nwmain"
    } else {
        "bin/win32/nwmain.exe"
    };
    Some(root.join(rel)).filter(|p| p.is_file())
}

/// The module's name for `+TestNewModule`, if `module` (a `.mod` file or a
/// module folder) is in the game's modules folder `modules` (the user
/// directory's, or the one `nwn.ini` aliases: `GameInstall::user_folder`).
pub fn module_name(modules: &Path, module: &Path) -> Option<String> {
    let folder = module.parent()?;
    let same = |a: &Path, b: &Path| {
        a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b)
    };
    if !same(folder, modules) {
        return None;
    }
    let is_mod = module.extension().is_some_and(|e| e.eq_ignore_ascii_case("mod"));
    let name = if is_mod { module.file_stem()? } else { module.file_name()? };
    Some(name.to_string_lossy().into_owned())
}

/// The game's number on Steam.
const STEAM_APP: &str = "704450";

/// The environment variables that tell a Steam copy of the game it is
/// Steam's.
const STEAM_VARS: [&str; 2] = ["SteamAppId", "SteamGameId"];

/// Whether the client is a Steam copy of the game: in a Steam library's
/// `steamapps` (`SteamApps` in older Windows libraries).
pub fn steam_copy(client: &Path) -> bool {
    client.ancestors().any(|a| a.file_name().is_some_and(|n| n.eq_ignore_ascii_case("steamapps")))
}

/// The command Test Module runs: the client, from its own folder. The game
/// starts the module with the first character of the list
/// (`+TestNewModule`, as Aurora's F9 does), or, with `choose`, shows its
/// character selection for the module (`+LoadNewModule`). With `steam`, a
/// Steam copy is told it is Steam's: Moonglow then reads the Workshop's
/// content too (Options › Folders' Read Steam Workshop content,
/// [`crate::Settings::test_through_steam`]).
pub fn command(client: &Path, user_dir: &Path, module: &str, choose: bool, steam: bool) -> Command {
    let mut c = Command::new(client);
    if let Some(dir) = client.parent() {
        c.current_dir(dir);
    }
    let how = if choose { "+LoadNewModule" } else { "+TestNewModule" };
    c.arg("-userdirectory").arg(user_dir).arg(how).arg(module);
    // A Steam copy told which game it is reaches Steam when Steam is
    // running and is expected to load what Steam brings it (the Workshop
    // items subscribed to: overrides of the game's textures, materials),
    // as when started from Steam. Started without the app ID (as Aurora
    // starts it) it is expected to run without Steam's Workshop content,
    // so that the test is nearer what Moonglow's views show (a builder's
    // report of 2026-11-22; the owner's ruling). Not tried with Steam
    // running: the client here only ever runs with Steam hidden. Only the
    // two variables are taken away: what else a Moonglow started from
    // Steam inherits (the overlay's LD_PRELOAD) is passed on.
    // (That the client has no relaunch through Steam rests on reading the
    // binaries' strings, not on a run: they load Steam's library
    // themselves and name no RestartAppIfNecessary. A bare start is how
    // the tests here run the Steam copy, Steam hidden.)
    if steam_copy(client) {
        if steam {
            for var in STEAM_VARS {
                c.env(var, STEAM_APP);
            }
        } else {
            // (Not passed on from Moonglow's own, where Steam started it.)
            for var in STEAM_VARS {
                c.env_remove(var);
            }
        }
    }
    c
}

/// The log's line for a test of a Steam copy of the game, which says
/// whether it was started through Steam and why; none for
/// another copy (GOG's, Beamdog's), where Steam has no part.
pub fn steam_line(client: &Path, steam: bool) -> Option<&'static str> {
    if !steam_copy(client) {
        return None;
    }
    Some(if steam {
        "Test Module: started through Steam, as Read Steam Workshop content is on \
         (Options › Folders)."
    } else {
        "Test Module: started without Steam, as Read Steam Workshop content is off \
         (Options › Folders)."
    })
}

/// The module Test From Here writes into the modules folder (and replaces
/// each time).
pub const FROM_HERE: &str = "moonglow-test";

/// Lets a program Moonglow started run on its own: a thread waits for it
/// to end, so that the system forgets it then. (A program its parent never
/// waits for stays listed, as a "defunct" process, until the parent ends:
/// the game, after Test Module, for as long as Moonglow ran.)
pub(crate) fn let_run(mut child: std::process::Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

/// The games started for a test that are still running, by process id.
static GAMES: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

/// [`let_run`] for the game started for a test: known while it runs.
pub(crate) fn let_game_run(mut child: std::process::Child) {
    let pid = child.id();
    GAMES.lock().unwrap_or_else(|e| e.into_inner()).push(pid);
    std::thread::spawn(move || {
        let _ = child.wait();
        GAMES.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| *p != pid);
    });
}

/// Whether a game Moonglow started for a test is still running. A second
/// test would write the test module under it (Test From Here writes it
/// anew each time), and the game then reports a corrupt module.
pub(crate) fn game_running() -> bool {
    !GAMES.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
}

/// The game client's program names, on the three systems.
pub(crate) const CLIENT_NAMES: [&str; 3] = ["nwmain-linux", "nwmain", "nwmain.exe"];

/// A program running on this machine, found by its name ([`running`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Running {
    pub pid: u32,
    /// Its arguments, where the system tells another program's (Linux and
    /// macOS; not Windows).
    pub args: Option<Vec<String>>,
}

impl Running {
    /// Whether it was started on the module `name` (`+TestNewModule` or
    /// `+LoadNewModule` and the name, as a test starts the game); `None`
    /// where its arguments are not known.
    pub(crate) fn plays(&self, name: &str) -> Option<bool> {
        let args = self.args.as_ref()?;
        let start = |a: &str| a == "+TestNewModule" || a == "+LoadNewModule";
        if args.windows(2).any(|w| start(&w[0]) && w[1].eq_ignore_ascii_case(name)) {
            return Some(true);
        }
        // (macOS's `ps` gives the command line as one, its arguments by
        // spaces, a name's own spaces too: the name then ends it, or is
        // followed by another option.)
        let line = args.join(" ").to_lowercase();
        let name = name.to_lowercase();
        let plays = ["+testnewmodule ", "+loadnewmodule "].iter().any(|start| {
            line.match_indices(start).any(|(at, _)| {
                let rest = &line[at + start.len()..];
                rest.strip_prefix(name.as_str())
                    .is_some_and(|r| r.is_empty() || r.starts_with(" +") || r.starts_with(" -"))
            })
        });
        Some(plays)
    }
}

/// How long the system's own tool that lists what runs (`ps`, `tasklist`)
/// is waited for: `mg_core::process::output_within` stops it then.
#[cfg(any(target_os = "macos", windows))]
const SYSTEM_TOOL: std::time::Duration = std::time::Duration::from_secs(10);

/// The programs running whose name is one of `names` (case aside), as the
/// system lists them; read only: nothing is ever stopped (the user may be
/// playing). Empty where the list can't be read.
pub(crate) fn running(names: &[&str]) -> Vec<Running> {
    let named = |n: &str| names.iter().any(|m| m.eq_ignore_ascii_case(n));
    let mut out = Vec::new();
    #[cfg(target_os = "linux")]
    {
        let Ok(entries) = std::fs::read_dir("/proc") else { return out };
        for e in entries.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            // (By the program's file, or by the name it was started by,
            // `comm`, cut at 15 bytes: a link's.)
            let exe = std::fs::read_link(e.path().join("exe")).ok();
            let exe = exe.as_deref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy());
            let comm = std::fs::read_to_string(e.path().join("comm")).unwrap_or_default();
            if !exe.is_some_and(|n| named(n.trim_end_matches(" (deleted)")))
                && !named(comm.trim_end())
            {
                continue;
            }
            let args = std::fs::read(e.path().join("cmdline")).ok().map(|b| {
                b.split(|c| *c == 0)
                    .filter(|a| !a.is_empty())
                    .map(|a| String::from_utf8_lossy(a).into_owned())
                    .collect()
            });
            out.push(Running { pid, args });
        }
    }
    #[cfg(target_os = "macos")]
    {
        // (`ps`'s accounting name, the program's own without its path,
        // which has spaces: `Neverwinter Nights`; then its command line.)
        // (Within a time: one that hangs held the window.)
        let mut ps = Command::new("ps");
        ps.args(["-ax", "-o", "pid=,ucomm=,args="]);
        let Ok(ps) = mg_core::process::output_within(&mut ps, SYSTEM_TOOL) else {
            return out;
        };
        for line in String::from_utf8_lossy(&ps.stdout).lines() {
            if let Some((pid, name, args)) = ps_line(line)
                && named(name)
            {
                out.push(Running { pid, args: Some(vec![args.to_string()]) });
            }
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // (No console window flashing up for it.)
        const NO_WINDOW: u32 = 0x0800_0000;
        // (Within a time: one that hangs held the window.)
        let mut list = Command::new("tasklist");
        list.args(["/FO", "CSV", "/NH"]).creation_flags(NO_WINDOW);
        let Ok(list) = mg_core::process::output_within(&mut list, SYSTEM_TOOL) else {
            return out;
        };
        for line in String::from_utf8_lossy(&list.stdout).lines() {
            let cells: Vec<&str> = line.split("\",\"").map(|c| c.trim_matches('"')).collect();
            if let [name, pid, ..] = cells[..]
                && named(name)
                && let Ok(pid) = pid.parse()
            {
                out.push(Running { pid, args: None });
            }
        }
    }
    let _ = &named;
    out
}

/// A line of `ps -o pid=,ucomm=,args=`: the process, its name and its
/// command line, the columns padded with spaces.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn ps_line(line: &str) -> Option<(u32, &str, &str)> {
    let word = |s: &str| -> (usize, usize) {
        let start = s.len() - s.trim_start().len();
        let end = s[start..].find(char::is_whitespace).map_or(s.len(), |e| start + e);
        (start, end)
    };
    let (a, b) = word(line);
    let pid = line[a..b].parse().ok()?;
    let rest = &line[b..];
    let (c, d) = word(rest);
    (c < d).then(|| (pid, &rest[c..d], rest[d..].trim()))
}

/// The game clients running that Moonglow did not start for a test (one
/// the user started, or one left by an earlier session of Moonglow).
pub(crate) fn other_games() -> Vec<Running> {
    let own = GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone();
    running(&CLIENT_NAMES).into_iter().filter(|g| !own.contains(&g.pid)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program running is found by its name, with its arguments where
    /// the system tells them, and is said to play the module its
    /// `+TestNewModule` names (GitHub issue 6: a game Moonglow did not
    /// start had the test module written under it).
    #[cfg(unix)]
    #[test]
    fn a_running_program_is_found_by_its_name() {
        // (A shell that stays, with the arguments a test gives the game.)
        let mut child = Command::new("sh")
            .args(["-c", "sleep 30; true", "sh", "+TestNewModule", "moonglow-test"])
            .spawn()
            .unwrap();
        // (macOS's `sh` runs as bash or another shell, by that name in
        // `ps`'s accounting.)
        let found = running(&["SH", "BASH", "DASH", "ZSH"]);
        let me = found.iter().find(|r| r.pid == child.id()).cloned();
        child.kill().unwrap();
        child.wait().unwrap();
        let me = me.unwrap_or_else(|| panic!("not found: {found:?}"));
        assert_eq!(me.plays("Moonglow-Test"), Some(true));
        assert_eq!(me.plays("other"), Some(false));
        assert!(running(&["no-such-program-of-moonglow"]).is_empty());
        let unknown = Running { pid: 1, args: None };
        assert_eq!(unknown.plays("moonglow-test"), None);
    }

    /// `ps`'s padded columns read (macOS), a path with spaces in the
    /// command line kept whole for the module's name to be found in it.
    #[test]
    fn a_line_of_ps_is_read() {
        let line = "  4242 nwmain           /Users/a/Neverwinter Nights/bin/macos/nwmain.app/Contents/\
                    MacOS/nwmain -userdirectory /Users/a/nwn +TestNewModule My Module";
        let (pid, name, args) = ps_line(line).unwrap();
        assert_eq!((pid, name), (4242, "nwmain"));
        let game = Running { pid, args: Some(vec![args.to_string()]) };
        assert_eq!(game.plays("my module"), Some(true));
        assert_eq!(game.plays("My"), Some(false));
        assert_eq!(ps_line("  17 "), None);
    }

    #[cfg(windows)]
    #[test]
    fn a_running_program_is_found_by_its_name() {
        let mut child = Command::new("ping").args(["-n", "30", "127.0.0.1"]).spawn().unwrap();
        let found = running(&["ping.exe"]);
        let me = found.iter().any(|r| r.pid == child.id());
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(me, "not found: {found:?}");
    }

    /// A program let run leaves nothing behind when it ends: no "defunct"
    /// entry waiting for Moonglow to collect it.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_program_let_run_is_forgotten_when_it_ends() {
        let child = Command::new("true").spawn().unwrap();
        let entry = std::path::PathBuf::from(format!("/proc/{}", child.id()));
        let_run(child);
        // (It ends at once; the thread that waits for it runs soon after.)
        for _ in 0..200 {
            if !entry.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let state = std::fs::read_to_string(entry.join("stat")).unwrap_or_default();
        panic!("still listed: {state}");
    }

    #[test]
    fn the_module_is_named_from_the_modules_folder() {
        let dir = mg_testkit::scratch_dir("mg-test-module");
        let modules = dir.join("modules");
        std::fs::create_dir_all(modules.join("Folder Module")).unwrap();
        std::fs::write(modules.join("My Module.mod"), b"").unwrap();
        let named = |m: &Path| module_name(&modules, m);
        assert_eq!(named(&modules.join("My Module.mod")).as_deref(), Some("My Module"));
        assert_eq!(named(&modules.join("Folder Module")).as_deref(), Some("Folder Module"));
        assert_eq!(named(&dir.join("elsewhere.mod")), None);
        let c =
            command(Path::new("/game/bin/linux-x86/nwmain-linux"), &dir, "My Module", false, true);
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            [
                "-userdirectory".to_string(),
                dir.display().to_string(),
                "+TestNewModule".into(),
                "My Module".into()
            ]
        );
        assert_eq!(c.get_current_dir(), Some(Path::new("/game/bin/linux-x86")));
        let c =
            command(Path::new("/game/bin/linux-x86/nwmain-linux"), &dir, "My Module", true, true);
        assert_eq!(c.get_args().nth(2).unwrap(), "+LoadNewModule");
        // A copy outside Steam is told nothing of Steam, either way.
        assert_eq!(c.get_envs().count(), 0);
        let other = Path::new("/game/bin/linux-x86/nwmain-linux");
        assert_eq!(command(other, &dir, "My Module", false, false).get_envs().count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// What a test of a Steam copy is told: through Steam (Read Steam
    /// Workshop content on), the game's Steam number in both variables;
    /// without (off), both taken away, so that none of Moonglow's own
    /// reaches the game; a copy outside Steam is told nothing either way.
    /// The arguments are the same both ways.
    #[test]
    fn a_steam_copy_is_started_through_steam_only_when_asked() {
        let dir = Path::new("/home/me/nwn");
        let envs = |c: &Command| -> Vec<(String, Option<String>)> {
            let mut all: Vec<_> = (c.get_envs())
                .map(|(k, v)| {
                    (k.to_string_lossy().into_owned(), v.map(|v| v.to_string_lossy().into_owned()))
                })
                .collect();
            all.sort();
            all
        };
        let args = |c: &Command| -> Vec<String> {
            c.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
        };
        for steam in [
            "/lib/steamapps/common/Neverwinter Nights/bin/linux-x86/nwmain-linux",
            r"C:\Program Files (x86)\Steam\SteamApps\common\Neverwinter Nights\bin\win32\nwmain.exe",
        ] {
            let steam = Path::new(steam);
            // (A Windows path is split at `\` on Windows alone.)
            if !cfg!(windows) && steam.to_string_lossy().contains('\\') {
                assert!(!steam_copy(steam), "one name on Linux");
                continue;
            }
            assert!(steam_copy(steam), "{}", steam.display());
            let through = command(steam, dir, "My Module", false, true);
            let app = Some("704450".to_string());
            assert_eq!(
                envs(&through),
                [("SteamAppId".into(), app.clone()), ("SteamGameId".into(), app)]
            );
            let bare = command(steam, dir, "My Module", false, false);
            assert_eq!(envs(&bare), [("SteamAppId".into(), None), ("SteamGameId".into(), None)]);
            assert_eq!(args(&through), args(&bare));
            assert_eq!(args(&bare)[2], "+TestNewModule");
        }
        assert!(steam_copy(Path::new("/x/SteamApps/common/nwn/bin/macos/nwmain")));
        assert!(!steam_copy(Path::new("/opt/gog/Neverwinter Nights/bin/linux-x86/nwmain-linux")));
    }

    /// The log says how a test of a Steam copy started, naming the option
    /// as Options shows it; of another copy it says nothing of Steam.
    #[test]
    fn the_log_says_how_a_steam_copy_was_started() {
        let steam =
            Path::new("/lib/steamapps/common/Neverwinter Nights/bin/linux-x86/nwmain-linux");
        let through = steam_line(steam, true).unwrap();
        let bare = steam_line(steam, false).unwrap();
        assert!(through.starts_with("Test Module: started through Steam"), "{through}");
        assert!(bare.starts_with("Test Module: started without Steam"), "{bare}");
        for line in [through, bare] {
            assert!(line.contains(", as Read Steam Workshop content is o"), "{line}");
        }
        assert!(through.ends_with("is on (Options › Folders)."), "{through}");
        assert!(bare.ends_with("is off (Options › Folders)."), "{bare}");
        let other = Path::new("/opt/gog/Neverwinter Nights/bin/linux-x86/nwmain-linux");
        assert_eq!(steam_line(other, true), None);
        assert_eq!(steam_line(other, false), None);
    }
}
