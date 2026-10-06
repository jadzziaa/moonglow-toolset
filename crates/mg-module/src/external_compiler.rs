//! An external NWScript compiler (Options › Script Editor) in place of the
//! built-in one: `nwn_script_comp`, `nwnsc` or another, run as a program.
//!
//! The module's scripts are written to a scratch folder, the compiler is
//! run on those asked for, and what it made (`.ncs`, `.ndb`) is read back,
//! with what it said of each script that it made nothing for.
//!
//! How it is run is a line of arguments with places for what Moonglow
//! knows ([`PLACES`]); the two compilers builders use have theirs built in
//! ([`ExternalCompiler::default_arguments`]).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

/// The places in a line of arguments, and what goes in each.
pub const PLACES: [(&str, &str); 6] = [
    ("{files}", "the scripts to compile (an argument each)"),
    ("{src}", "the folder the module's scripts are written to"),
    ("{out}", "the folder the compiled scripts are read from"),
    ("{game}", "the game's folder"),
    ("{user}", "the NWN user folder"),
    ("{haks}", "the module's haks, comma-separated"),
];

/// The most characters of script paths given to one run of the compiler
/// (Windows takes about 32,000 in a command line).
const FILES_AT_ONCE: usize = 16_000;

/// An external compiler and what it is told.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalCompiler {
    pub program: PathBuf,
    /// The line of arguments; empty: the program's own
    /// ([`ExternalCompiler::default_arguments`]).
    pub arguments: String,
    pub game: Option<PathBuf>,
    pub user: Option<PathBuf>,
    /// The module's hak files, first listed first.
    pub haks: Vec<PathBuf>,
    /// Debug information (`.ndb`) is asked for (in the built-in lines).
    pub debug: bool,
}

/// What the compiler made of one script.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The script's name (its resref, lower case).
    pub name: String,
    pub ncs: Option<Vec<u8>>,
    pub ndb: Option<Vec<u8>>,
    /// What the compiler said of it, when it made no `.ncs`.
    pub message: String,
}

impl ExternalCompiler {
    /// The arguments for a compiler known by its file's name: `nwnsc`, else
    /// neverwinter.nim's `nwn_script_comp` (whose options other builds of
    /// the official compiler tend to follow).
    pub fn default_arguments(program: &Path, debug: bool) -> String {
        let name = program.file_stem().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
        let line = if name.contains("nwnsc") {
            "-q -y -o -n {game} -i {src} -b {out} {files}"
        } else {
            "-y --root {game} --userdirectory {user} --erfs {haks} --dirs {src} -d {out} -c {files}"
        };
        if debug { format!("-g {line}") } else { line.to_string() }
    }

    /// The arguments of one run: the line's words (double quotes keep
    /// spaces), each place filled in. A word whose place has nothing to
    /// give (no game folder, no haks) is left out, with the option before
    /// it.
    fn arguments(&self, src: &Path, out: &Path, files: &[PathBuf]) -> Vec<String> {
        let line = if self.arguments.trim().is_empty() {
            Self::default_arguments(&self.program, self.debug)
        } else {
            self.arguments.clone()
        };
        let text = |p: &Path| p.display().to_string();
        let haks = self.haks.iter().map(|h| text(h)).collect::<Vec<_>>().join(",");
        let values: [(&str, Option<String>); 5] = [
            ("{src}", Some(text(src))),
            ("{out}", Some(text(out))),
            ("{game}", self.game.as_deref().map(text)),
            ("{user}", self.user.as_deref().map(text)),
            ("{haks}", (!haks.is_empty()).then_some(haks)),
        ];
        let mut made: Vec<String> = Vec::new();
        for word in words(&line) {
            if word == "{files}" {
                made.extend(files.iter().map(|f| text(f)));
                continue;
            }
            let mut filled = word.clone();
            let mut missing = false;
            for (place, value) in &values {
                if filled.contains(place) {
                    match value {
                        Some(v) => filled = filled.replace(place, v),
                        None => missing = true,
                    }
                }
            }
            if missing {
                // (`--root {game}` without a game folder: neither word.)
                if made.last().is_some_and(|w| w.starts_with('-')) && !word.starts_with('-') {
                    made.pop();
                }
                continue;
            }
            made.push(filled);
        }
        made
    }

    /// Compiles `names` (resrefs, lower case) among `sources` (every script
    /// the module has: name and text). What was made of each, in order; or
    /// why the compiler could not be run at all.
    pub fn compile(
        &self,
        sources: &[(String, &[u8])],
        names: &[String],
    ) -> Result<Vec<Outcome>, String> {
        let dir = Scratch::new()?;
        let (src, out) = (dir.0.join("src"), dir.0.join("out"));
        let io = |what: &Path, e: std::io::Error| format!("{}: {e}", what.display());
        for d in [&src, &out] {
            std::fs::create_dir_all(d).map_err(|e| io(d, e))?;
        }
        for (name, text) in sources {
            let path = src.join(format!("{name}.nss"));
            std::fs::write(&path, text).map_err(|e| io(&path, e))?;
        }
        let files: Vec<PathBuf> = names.iter().map(|n| src.join(format!("{n}.nss"))).collect();
        let mut said = String::new();
        let mut from = 0;
        while from < files.len() {
            // As many scripts a run as a command line takes.
            let mut to = from;
            let mut length = 0;
            while to < files.len() && (to == from || length < FILES_AT_ONCE) {
                length += files[to].as_os_str().len() + 1;
                to += 1;
            }
            let mut command = Command::new(&self.program);
            command.args(self.arguments(&src, &out, &files[from..to])).current_dir(&src);
            quiet(&mut command);
            let run = command
                .output()
                .map_err(|e| format!("{} could not be run: {e}", self.program.display()))?;
            said.push_str(&String::from_utf8_lossy(&run.stdout));
            said.push_str(&String::from_utf8_lossy(&run.stderr));
            if !run.status.success() && !said.ends_with('\n') {
                said.push('\n');
            }
            from = to;
        }
        let made = made_files(&out);
        let read = |name: &str, extension: &str| {
            let file = made.iter().find(|(stem, e, _)| stem == name && e == extension)?;
            std::fs::read(&file.2).ok()
        };
        let messages = messages(&said, names);
        Ok(names
            .iter()
            .zip(messages)
            .map(|(name, message)| {
                let ncs = read(name, "ncs");
                let message = match (&ncs, message.is_empty()) {
                    (Some(_), _) => String::new(),
                    (None, false) => message,
                    (None, true) => format!(
                        "{name}.nss: {} made no compiled script of it",
                        self.program.file_name().unwrap_or_default().to_string_lossy()
                    ),
                };
                Outcome { name: name.clone(), ncs, ndb: read(name, "ndb"), message }
            })
            .collect())
    }
}

/// The files in `dir`: stem and extension (lower case), and path.
fn made_files(dir: &Path) -> Vec<(String, String, PathBuf)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
            let extension = path.extension()?.to_str()?.to_ascii_lowercase();
            Some((stem, extension, path))
        })
        .collect()
}

/// A line of arguments as words: split at spaces, double quotes keeping
/// what is between them together.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut any) = (false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            c => {
                word.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(word);
    }
    out
}

/// What the compiler said of each of `names`, from all it wrote: the lines
/// that name the script's file and speak of an error, from the file's name
/// on (`broken.nss(4): ERROR: …`, which is how the built-in compiler says
/// it, too). An error in an include is its includer's: the line names both
/// (nwn_script_comp), or follows the line that names the script (nwnsc).
fn messages(said: &str, names: &[String]) -> Vec<String> {
    let mut out = vec![String::new(); names.len()];
    let mut current: Option<usize> = None;
    for line in said.lines() {
        let lower = line.to_ascii_lowercase();
        // The script the line is about: the one whose file it names first.
        let named = names
            .iter()
            .enumerate()
            .filter_map(|(i, n)| find_file(&lower, n).map(|at| (at, i)))
            .min();
        if let Some((_, i)) = named {
            current = Some(i);
        }
        // ("error" as a word: not a summary's "2 errored".)
        let error = lower.match_indices("error").any(|(at, word)| {
            !lower[at + word.len()..].chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        });
        if !error {
            continue;
        }
        let Some(i) = named.map(|(_, i)| i).or(current) else { continue };
        // From where a file and its line are given, if they are.
        let from = lower
            .match_indices(".nss(")
            .next()
            .map(|(dot, _)| {
                lower[..dot]
                    .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .map_or(0, |p| p + 1)
            })
            .unwrap_or(0);
        let text = line[from..].trim();
        if !out[i].is_empty() {
            out[i].push('\n');
        }
        out[i].push_str(text);
    }
    out
}

/// Where `line` (lower case) names the file of script `name`: `name.nss`
/// as a whole word.
fn find_file(line: &str, name: &str) -> Option<usize> {
    let file = format!("{name}.nss");
    line.match_indices(&file).map(|(at, _)| at).find(|&at| {
        !line[..at].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// No console window for the compiler, on Windows.
fn quiet(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = command;
}

/// A scratch folder of this run's own, removed when done.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Scratch, String> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("moonglow-compile-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_of_arguments_is_filled_in() {
        let c = ExternalCompiler {
            program: "/opt/nwn_script_comp".into(),
            game: Some("/games/Neverwinter Nights".into()),
            haks: vec!["/haks/a.hak".into(), "/haks/b.hak".into()],
            ..Default::default()
        };
        let files = [PathBuf::from("/s/one.nss"), PathBuf::from("/s/two.nss")];
        let args = c.arguments(Path::new("/s"), Path::new("/o"), &files);
        assert_eq!(
            args,
            [
                "-y",
                "--root",
                "/games/Neverwinter Nights",
                // (No user folder: `--userdirectory {user}` is left out.)
                "--erfs",
                "/haks/a.hak,/haks/b.hak",
                "--dirs",
                "/s",
                "-d",
                "/o",
                "-c",
                "/s/one.nss",
                "/s/two.nss"
            ]
        );
        // nwnsc by its name; a line of one's own, with a quoted word.
        let nwnsc = ExternalCompiler { program: "C:/tools/NWNSC.exe".into(), ..c.clone() };
        assert_eq!(
            nwnsc.arguments(Path::new("/s"), Path::new("/o"), &files)[..4],
            ["-q", "-y", "-o", "-n"]
        );
        let own = ExternalCompiler { arguments: "--x \"a b\" -o={out} {files}".into(), ..c };
        assert_eq!(
            own.arguments(Path::new("/s"), Path::new("/o"), &files[..1]),
            ["--x", "a b", "-o=/o", "/s/one.nss"]
        );
    }

    #[test]
    fn messages_go_to_the_script_they_are_about() {
        let names = ["broken".to_string(), "user".to_string(), "fine".to_string()];
        // nwn_script_comp's lines.
        let said = "E [2026-10-06T14:59:33] [1/3] src/broken.nss: broken.nss(1): ERROR: UNDEFINED \
                    IDENTIFIER (Nope) [<0ms]\nE [x] [2/3] src/user.nss: inc.nss(3): ERROR: PARSING \
                    VARIABLE LIST\nI [x] 1 successful, 0 skipped, 2 errored\n";
        let m = messages(said, &names);
        assert_eq!(m[0], "broken.nss(1): ERROR: UNDEFINED IDENTIFIER (Nope) [<0ms]");
        assert_eq!(m[1], "inc.nss(3): ERROR: PARSING VARIABLE LIST");
        assert_eq!(m[2], "");
        // nwnsc's: the script named, then its errors.
        let said = "Compiling: user.nss\ninc.nss(3): Error: NSC1040: Syntax error\nCompiling: \
                    fine.nss\n";
        let m = messages(said, &names);
        assert_eq!(m[1], "inc.nss(3): Error: NSC1040: Syntax error");
        assert_eq!((m[0].as_str(), m[2].as_str()), ("", ""));
    }

    /// A stand-in compiler (a shell script): it "compiles" a script with
    /// `main` in it by copying it, and complains of the others.
    #[cfg(unix)]
    #[test]
    fn a_compiler_is_run_and_what_it_made_read_back() {
        use std::os::unix::fs::PermissionsExt;
        let dir =
            std::env::temp_dir().join(format!("moonglow-extcomp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("fakecomp");
        std::fs::write(
            &program,
            "#!/bin/sh\nout=$1; shift\nfor f in \"$@\"; do b=$(basename \"$f\" .nss)\n  if grep -q \
             main \"$f\"; then cp \"$f\" \"$out/$b.ncs\"\n  else echo \"$b.nss(1): ERROR: NO \
             MAIN\" >&2; fi\ndone\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let c =
            ExternalCompiler { program, arguments: "{out} {files}".into(), ..Default::default() };
        let sources: [(String, &[u8]); 2] =
            [("good".into(), b"void main() {}"), ("bad".into(), b"int x;")];
        let made = c.compile(&sources, &["good".into(), "bad".into()]).unwrap();
        assert_eq!(made[0].ncs.as_deref(), Some(&b"void main() {}"[..]));
        assert_eq!(
            (made[1].ncs.as_ref(), made[1].message.as_str()),
            (None, "bad.nss(1): ERROR: NO MAIN")
        );
        // One that isn't there says so, once.
        let missing = ExternalCompiler { program: dir.join("nothing"), ..Default::default() };
        assert!(
            missing.compile(&sources, &["good".into()]).unwrap_err().contains("could not be run")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
