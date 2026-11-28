use super::*;
use crate::Module;
use mg_core::ResRef;
use mg_resman::ResKey;

/// A fake NWNX home in a scratch folder: `Binaries/` with empty `.so`
/// files for `plugins` (Core's always), `NWScript/` with two includes.
/// (Absolute, as NWNX's home is taken: on Windows that also settles the
/// scratch folder's `..` and its mixed slashes.)
fn fake_home(dir: &Path, plugins: &[&str]) -> PathBuf {
    let home = std::path::absolute(dir.join("nwnx")).unwrap();
    let bin = home.join("Binaries");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(home.join("NWScript")).unwrap();
    for p in std::iter::once("Core").chain(plugins.iter().copied()) {
        std::fs::write(bin.join(format!("NWNX_{p}.so")), b"").unwrap();
    }
    for (inc, plugin) in [("nwnx_admin", "Administration"), ("nwnx_store", "Store")] {
        let text = format!("const string NWNX_{plugin} = \"NWNX_{plugin}\";\nvoid f();\n");
        std::fs::write(home.join("NWScript").join(format!("{inc}.nss")), text).unwrap();
    }
    std::fs::write(home.join("NWScript/nwnx_consts.nss"), "const int X = 1;\n").unwrap();
    home
}

/// A project folder with these files (`moonglow.toml`, `.moonglow/user.toml`,
/// an env file), read.
fn project(dir: &Path, team: &str, own: Option<&str>, env: Option<&str>) -> ProjectFile {
    std::fs::create_dir_all(dir.join(".moonglow")).unwrap();
    std::fs::write(dir.join(".moonglow/.gitignore"), "*\n").unwrap();
    std::fs::write(dir.join("moonglow.toml"), team).unwrap();
    match own {
        Some(own) => std::fs::write(dir.join(".moonglow/user.toml"), own).unwrap(),
        None => {
            let _ = std::fs::remove_file(dir.join(".moonglow/user.toml"));
        }
    }
    if let Some(env) = env {
        std::fs::write(dir.join("nwnx.env"), env).unwrap();
    }
    ProjectFile::read(dir)
}

fn toml_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

fn levels(r: &Resolved) -> Vec<(Level, &str)> {
    r.notes.iter().map(|n| (n.level, n.text.as_str())).collect()
}

fn linux() -> Options<'static> {
    Options { linux: true, ..Default::default() }
}

/// `[nwnx]` from either file: the team's use and home a default, the
/// builder's own over them; an unknown key and a wrong kind said.
#[test]
fn the_settings_are_read_from_either_file() {
    let mut p = ProjectFile::default();
    p.apply("moonglow.toml", "[nwnx]\nuse = true\nhome = \"/team/nwnx\"\nenv = \"a.env\"\n", false);
    p.apply("user.toml", "[nwnx]\nhome = \"/mine/nwnx\"\nenv = \"b.env\"\n", true);
    assert_eq!(p.nwnx.use_nwnx, Some(true));
    assert_eq!(p.nwnx.team_home.as_deref(), Some("/team/nwnx"));
    assert_eq!(p.nwnx.own_home.as_deref(), Some("/mine/nwnx"));
    assert_eq!(p.nwnx.env.as_deref(), Some("b.env"));
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    p.apply("user.toml", "[nwnx]\nuse = \"yes\"\nfolder = \"x\"\n", true);
    assert_eq!(p.warnings.len(), 2, "{:?}", p.warnings);
    assert!(p.warnings[0].contains("use is true or false"), "{:?}", p.warnings);
    assert!(p.warnings[1].contains("\"folder\" is not a key of [nwnx]"), "{:?}", p.warnings);
}

/// Who turns NWNX on: the team's `use = true` for a builder with a home
/// (the team's default, or their own, which wins); `use = false` of their
/// own turns it off; `--no-nwnx` too; `--nwnx DIR` whatever the files say.
#[test]
fn nwnx_is_on_for_a_builder_with_a_home() {
    let dir = mg_testkit::scratch_dir("nwnx-who");
    let home = fake_home(&dir, &["Util"]);
    let other = fake_home(&dir.join("other"), &["Util"]);
    let root = dir.join("project");
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\n", toml_path(&home));
    let r = resolve(&project(&root, &team, None, None), &linux());
    assert_eq!(r.nwnx.as_ref().map(|n| n.home.clone()), Some(home.clone()), "{:?}", r.notes);
    // The builder's own home wins.
    let own = format!("[nwnx]\nhome = \"{}\"\n", toml_path(&other));
    let r = resolve(&project(&root, &team, Some(&own), None), &linux());
    assert_eq!(r.nwnx.unwrap().home, other);
    // The builder opts out.
    let r = resolve(&project(&root, &team, Some("[nwnx]\nuse = false\n"), None), &linux());
    assert!(r.nwnx.is_none() && r.notes.is_empty(), "{:?}", r.notes);
    // --no-nwnx; --nwnx DIR over a project that doesn't use it.
    let p = project(&root, &team, None, None);
    let off = Options { choice: Choice::Off, ..linux() };
    assert!(resolve(&p, &off).nwnx.is_none());
    let p = project(&root, "", None, None);
    assert!(resolve(&p, &linux()).nwnx.is_none(), "no [nwnx]: none");
    let given = Options { choice: Choice::Home(other.clone()), ..linux() };
    assert_eq!(resolve(&p, &given).nwnx.unwrap().home, other);
    // use = true and no home anywhere: a warning, and no NWNX.
    let r = resolve(&project(&root, "[nwnx]\nuse = true\n", None, None), &linux());
    assert!(r.nwnx.is_none() && !r.failed());
    assert!(r.notes[0].text.contains("no home is set"), "{:?}", r.notes);
}

/// A home that isn't there, or has no core, stops the start; so does an
/// environment file that isn't there or isn't NAME=value lines.
#[test]
fn a_missing_home_core_or_env_file_is_an_error() {
    let dir = mg_testkit::scratch_dir("nwnx-missing");
    let root = dir.join("project");
    let gone = dir.join("no-such-nwnx");
    let r = resolve(
        &project(&root, "", None, None),
        &Options { choice: Choice::Home(gone.clone()), ..linux() },
    );
    assert!(r.failed() && r.nwnx.is_none());
    assert!(r.errors()[0].contains("is not there"), "{:?}", r.notes);
    let home = fake_home(&dir, &[]);
    std::fs::remove_file(home.join("Binaries/NWNX_Core.so")).unwrap();
    let given = Options { choice: Choice::Home(home.clone()), ..linux() };
    let r = resolve(&project(&root, "", None, None), &given);
    assert!(r.errors()[0].contains("NWNX_Core.so is not there"), "{:?}", r.notes);
    std::fs::write(home.join("Binaries/NWNX_Core.so"), b"").unwrap();
    let r = resolve(&project(&root, "[nwnx]\nenv = \"gone.env\"\n", None, None), &given);
    assert!(r.errors()[0].contains("gone.env can't be read"), "{:?}", r.notes);
    let r = resolve(
        &project(&root, "[nwnx]\nenv = \"nwnx.env\"\n", None, Some("not a line\n")),
        &given,
    );
    assert!(r.errors()[0].contains("line 1"), "{:?}", r.notes);
}

/// The server's environment: the core preloaded and the plugins' folder
/// first, then the file's lines (a relative load path of its made whole);
/// nothing else of the caller's.
#[test]
fn the_server_environment_is_moonglow_s_and_the_file_s() {
    let dir = mg_testkit::scratch_dir("nwnx-env");
    let home = fake_home(&dir, &["Util", "Tdod"]);
    let root = dir.join("project");
    let env = "NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\nNWNX_TDOD_SKIP=n\nNWNX_CORE_LOG_LEVEL=6\n";
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\nenv = \"nwnx.env\"\n", toml_path(&home));
    let r = resolve(&project(&root, &team, None, Some(env)), &linux());
    let n = r.nwnx.unwrap();
    let text: Vec<(String, String)> = (n.server_env().into_iter())
        .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
        .collect();
    let bin = home.join("Binaries");
    let want = [
        ("LD_PRELOAD", bin.join("NWNX_Core.so").display().to_string()),
        ("NWNX_CORE_LOAD_PATH", bin.display().to_string()),
        ("NWNX_CORE_SKIP_ALL", "y".into()),
        ("NWNX_UTIL_SKIP", "n".into()),
        ("NWNX_TDOD_SKIP", "n".into()),
        ("NWNX_CORE_LOG_LEVEL", "6".into()),
    ];
    let want: Vec<(String, String)> =
        want.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    assert_eq!(text, want);
    assert_eq!(n.plugins.iter().cloned().collect::<Vec<_>>(), ["Core", "Tdod", "Util"]);
    assert_eq!(n.env_file, Some(root.join("nwnx.env")));
    // NWNX_CORE_LOAD_PATH of the file's: whole, from the project.
    let env = "NWNX_CORE_LOAD_PATH=plugins\n";
    std::fs::create_dir_all(root.join("plugins")).unwrap();
    let n = resolve(&project(&root, &team, None, Some(env)), &linux()).nwnx.unwrap();
    assert_eq!(n.binaries, root.join("plugins"));
    let paths: Vec<_> =
        n.server_env().into_iter().filter(|(k, _)| k == "NWNX_CORE_LOAD_PATH").collect();
    assert_eq!(paths, [("NWNX_CORE_LOAD_PATH".into(), root.join("plugins").into())]);
}

/// NWNX's skip rules, as its core reads them: the plugin's own switch,
/// else SKIP_ALL; its words, case aside; Core always; Experimental only
/// when asked.
#[test]
fn plugins_are_skipped_as_nwnx_reads_the_settings() {
    let env = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    };
    let all = env(&[("NWNX_CORE_SKIP_ALL", "Yes"), ("NWNX_UTIL_SKIP", "N")]);
    assert!(skipped("Store", &all) && !skipped("Util", &all) && !skipped("Core", &all));
    assert!(!skipped("WebHook", &env(&[("NWNX_WEBHOOK_SKIP", "false")])));
    assert!(skipped("WebHook", &env(&[("NWNX_WEBHOOK_SKIP", "t")])));
    assert!(!skipped("Store", &env(&[])), "nothing set: every plugin loads");
    // A value NWNX doesn't read is no value: SKIP_ALL's.
    assert!(skipped("Util", &env(&[("NWNX_CORE_SKIP_ALL", "1"), ("NWNX_UTIL_SKIP", "maybe")])));
    assert!(skipped("Experimental", &env(&[])));
    let exp = env(&[("NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN", "true")]);
    assert!(!skipped("Experimental", &exp));
}

/// A plugin switched on that isn't in Binaries stops the start (the
/// team's own plugin too); one switched off that isn't there is nothing.
#[test]
fn a_plugin_switched_on_and_missing_is_an_error() {
    let dir = mg_testkit::scratch_dir("nwnx-plugin-missing");
    let home = fake_home(&dir, &["Util"]);
    let root = dir.join("project");
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\nenv = \"nwnx.env\"\n", toml_path(&home));
    let env = "NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\nNWNX_TDOD_SKIP=n\nNWNX_STORE_SKIP=y\n";
    let r = resolve(&project(&root, &team, None, Some(env)), &linux());
    assert!(r.nwnx.is_none());
    assert_eq!(r.errors().len(), 1, "{:?}", r.notes);
    // (NWNX reads a plugin's switch from its file's name, upper-cased:
    // the file a key is for can't be told by its case, so none is named.)
    let e = r.errors()[0];
    assert!(e.contains("NWNX_TDOD_SKIP=n switches a plugin on"), "{e}");
    assert!(
        e.contains(&format!(
            "no NWNX plugin library for TDOD in {}",
            home.join("Binaries").display()
        )),
        "{e}"
    );
    assert!(!e.contains("NWNX_TDOD.so"), "{e}");
}

/// On Windows and macOS a project that uses NWNX runs without it, and
/// says so.
#[test]
fn nwnx_is_linux_only() {
    let dir = mg_testkit::scratch_dir("nwnx-linux-only");
    let home = fake_home(&dir, &[]);
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\n", toml_path(&home));
    let p = project(&dir.join("project"), &team, None, None);
    let r = resolve(&p, &Options { linux: false, ..Default::default() });
    assert!(r.nwnx.is_none() && !r.failed());
    assert_eq!(levels(&r), [(Level::Warning, LINUX_ONLY)]);
    // No home either: still only that (a home would not help there).
    let p = project(&dir.join("project"), "[nwnx]\nuse = true\n", None, None);
    let r = resolve(&p, &Options { linux: false, ..Default::default() });
    assert_eq!(levels(&r), [(Level::Warning, LINUX_ONLY)]);
    // A project that doesn't use it says nothing.
    let p = project(&dir.join("project"), "", None, None);
    assert!(resolve(&p, &Options { linux: false, ..Default::default() }).notes.is_empty());
}

fn script(m: &mut Module, name: &str, src: &str) {
    m.set(
        ResKey::new(ResRef::from_bytes(name.as_bytes()).unwrap(), ResType::NSS),
        src.as_bytes().to_vec(),
    );
}

/// The scripts' warnings: a plugin they include that the settings skip
/// (named, with how many include it; the include read for its plugin's
/// name, nwnx_admin is Administration's); with NWNX off, the scripts that
/// call it; the cache of scripts while they are replaced as it runs.
#[test]
fn the_scripts_are_held_to_the_settings() {
    let dir = mg_testkit::scratch_dir("nwnx-scripts");
    let home = fake_home(&dir, &["Util", "Store", "Administration", "Optimizations"]);
    let mut m = Module::new();
    script(&mut m, "a", "#include \"nwnx_store\"\n#include \"nwnx_admin\"\nvoid main(){}\n");
    script(&mut m, "b", "// #include \"nwnx_util\"\n#include \"nwnx_store\"\nvoid main(){}\n");
    script(&mut m, "c", "#include \"nwnx_consts\"\nvoid main(){}\n");
    script(&mut m, "d", "void main(){ NWNXCall(\"NWNX_Util\", \"X\"); }\n");
    script(&mut m, "e", "void main(){ string s = \"NWNXCall\"; }\n");
    let root = dir.join("project");
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\nenv = \"nwnx.env\"\n", toml_path(&home));
    let env = "NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\nNWNX_OPTIMIZATIONS_SKIP=n\n\
               NWNX_OPTIMIZATIONS_CACHE_SCRIPTS=true\n";
    let p = project(&root, &team, None, Some(env));
    let r = resolve(&p, &Options { module: Some(&m), live: true, ..linux() });
    let texts: Vec<&str> = r.notes.iter().map(|n| n.text.as_str()).collect();
    assert!(r.nwnx.is_some() && !r.failed(), "{texts:?}");
    assert_eq!(texts.len(), 3, "{texts:?}");
    assert!(texts[0].starts_with("the plugin Administration is off (NWNX_CORE_SKIP_ALL=y and no NWNX_ADMINISTRATION_SKIP=n), and 1 script include nwnx_admin"), "{texts:?}");
    assert!(texts[1].starts_with("the plugin Store is off"), "{texts:?}");
    assert!(texts[1].contains("2 scripts include nwnx_store"), "{texts:?}");
    assert!(texts[2].starts_with("scripts keep their first version: development/ and NWNX_Util_AddScript won't replace them"), "{texts:?}");
    // Not live: no word of the cache.
    let r = resolve(&p, &Options { module: Some(&m), ..linux() });
    assert_eq!(r.notes.len(), 2, "{:?}", r.notes);
    // NWNX off: the scripts that call it (an include of NWNX's, or
    // NWNXCall; not one in a comment or a string).
    let r = resolve(&p, &Options { module: Some(&m), choice: Choice::Off, ..linux() });
    assert_eq!(
        levels(&r),
        [(
            Level::Warning,
            "4 scripts call NWNX, which is off: those calls return nothing (a, b, c and 1 more)"
        )]
    );
}

/// The server's lines: the two builds compared once both are told (an
/// error where the game build differs, a warning where its revision
/// does), NWNX's own mismatch and the loader's failed preload errors, a
/// plugin that failed to load a warning, one skipped by the settings
/// nothing (the scripts' check tells it before the start). (The fixtures are NWNX's own
/// format strings, `Core/NWNXCore.cpp`, filled in: a real mismatch can't
/// be made on one machine.)
#[test]
fn the_server_s_lines_tell_the_builds_and_failures() {
    let mut b = Builds::default();
    assert_eq!(b.line("Starting NWNX 8193.37-17 [2d8465a]"), None);
    assert_eq!(b.line("Neverwinter Nights Server"), None);
    let same = b.line("Build: 8193.37-17 [26c6e573]").unwrap();
    assert_eq!(same.level, Level::Info);
    assert_eq!(b.line("Build: 8193.37-17 [26c6e573]"), None, "said once");
    let mut b = Builds::default();
    b.line("Build: 8193.37-17 [26c6e573]");
    assert_eq!(b.line("Starting NWNX 8193.37-13 [abc]").unwrap().level, Level::Warning);
    let mut b = Builds::default();
    b.line("Starting NWNX 8193.36-12 [abc]");
    let n = b.line("Build: 8193.37-17 [26c6e573]").unwrap();
    assert_eq!(n.level, Level::Error);
    assert!(n.text.contains("8193.36-12") && n.text.contains("8193.37-17"), "{}", n.text);
    assert!(Builds::default().none_seen().is_some());
    let line = |l: &str| console_note(l, true).map(|n| n.level);
    assert_eq!(line("NWNX: Expected build 8193.36, got build 8193.37."), Some(Level::Error));
    assert_eq!(
        line("NWNX: Will terminate. Please use the correct NWNX build for your game version."),
        Some(Level::Error)
    );
    assert_eq!(
        line("NWNX: WARNING: POSTFIX VERSION MISMATCH: EXPECTED: 13, GOT: 17"),
        Some(Level::Warning)
    );
    let preload = "ERROR: ld.so: object '/x/NWNX_Core.so' from LD_PRELOAD cannot be preloaded (cannot open shared object file): ignored.";
    assert_eq!(line(preload), Some(Level::Error));
    assert_eq!(console_note(preload, false).map(|n| n.level), Some(Level::Error));
    let failed =
        "E [08:44:36] [NWNX_Core] [Plugin.cpp:44] Unable to load plugin '/x/NWNX_Tdod.so': bad";
    assert_eq!(line(failed), Some(Level::Warning));
    assert!(console_note(failed, true).unwrap().text.contains("Unable to load plugin"));
    let skip = "I [08:44:36] [NWNX_Core] [NWNXCore.cpp:356] Skipping plugin NWNX_Store due to configuration.";
    assert_eq!(line(skip), None, "as the settings say: told before the start");
    assert_eq!(line("Server: Module loaded"), None);
    assert_eq!(console_note("NWNX: Expected build 1.2, got build 3.4.", false), None);
}

/// The plugins in a folder as NWNX's loader takes them
/// (`Core/NWNXCore.cpp`, `InitialSetupPlugins`): every file whose name,
/// without its last extension only, starts with `NWNX_` (in that case);
/// Core and Experimental by their exact names.
#[test]
fn plugins_are_found_as_nwnx_s_loader_finds_them() {
    let dir = mg_testkit::scratch_dir("nwnx-loader-names");
    std::fs::create_dir_all(dir.join("NWNX_Folder.so")).unwrap();
    for f in [
        "NWNX_Util.so",
        "NWNX_Bare",
        "NWNX_Two.so.1",
        "NWNX_Win.dll",
        "nwnx_lower.so",
        "Other.so",
        "NWNX_.so",
    ] {
        std::fs::write(dir.join(f), b"").unwrap();
    }
    let found: Vec<String> = plugins_in(&dir).into_keys().collect();
    assert_eq!(found, ["Bare", "Two.so", "Util", "Win"]);
    let env = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    };
    let all = env(&[("NWNX_CORE_SKIP_ALL", "y")]);
    // Exact names: `NWNX_core.so` is a plugin of its own, skipped as any.
    assert!(!skipped("Core", &all) && skipped("core", &all));
    assert!(skipped("Experimental", &env(&[])));
    assert!(!skipped("experimental", &env(&[])), "not NWNX_Experimental: loaded as any");
}

/// Why a plugin is off, as the settings make it so: Experimental by the
/// switch it needs, not by a SKIP it doesn't read first.
#[test]
fn experimental_is_off_for_want_of_its_own_switch() {
    let none = BTreeMap::new();
    let why = why_skipped("Experimental", &none);
    assert!(why.contains("NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN"), "{why}");
    assert!(!why.contains("NWNX_EXPERIMENTAL_SKIP"), "{why}");
    let mut on: BTreeMap<String, String> = BTreeMap::new();
    on.insert("NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN".into(), "y".into());
    on.insert("NWNX_EXPERIMENTAL_SKIP".into(), "y".into());
    assert_eq!(why_skipped("Experimental", &on), "NWNX_EXPERIMENTAL_SKIP=y");
}

/// `mg serve --env` lines are checked with the project's: they come after
/// the file's (and win), a plugin they switch on is on (no "off" warning)
/// and must be there, and their relative `NWNX_CORE_LOAD_PATH` is made
/// whole from the current folder.
#[test]
fn more_lines_are_checked_with_the_project_s() {
    let dir = mg_testkit::scratch_dir("nwnx-more-env");
    let home = fake_home(&dir, &["Util", "Store"]);
    let root = dir.join("project");
    let team = format!("[nwnx]\nuse = true\nhome = \"{}\"\nenv = \"nwnx.env\"\n", toml_path(&home));
    let p = project(&root, &team, None, Some("NWNX_CORE_SKIP_ALL=y\nNWNX_UTIL_SKIP=n\n"));
    let mut m = Module::new();
    script(&mut m, "a", "#include \"nwnx_store\"\nvoid main(){}\n");
    let lines = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    };
    let o = Options { module: Some(&m), ..linux() };
    // Without them, Store is off; with them, on, and last in the env.
    let r = resolve(&p, &o);
    assert!(r.notes.iter().any(|n| n.text.starts_with("the plugin Store is off")), "{:?}", r.notes);
    let r = resolve_with(&p, &o, &lines(&[("NWNX_STORE_SKIP", "n")]));
    assert!(r.notes.is_empty(), "{:?}", r.notes);
    let n = r.nwnx.unwrap();
    assert!(n.loads("Store"));
    let last = n.server_env().pop().unwrap();
    assert_eq!(last, ("NWNX_STORE_SKIP".into(), "n".into()));
    // A plugin they switch on that isn't there: an error.
    let r = resolve_with(&p, &o, &lines(&[("NWNX_WEBHOOK_SKIP", "n")]));
    assert!(r.failed() && r.errors()[0].contains("WEBHOOK"), "{:?}", r.notes);
    // A relative load path: from the current folder (the crate's, under
    // cargo test: its `src`, no plugins in it), not the project's.
    let here = std::env::current_dir().unwrap();
    let p = project(&root, &team, None, Some("NWNX_CORE_LOG_LEVEL=6\n"));
    let r = resolve_with(&p, &o, &lines(&[("NWNX_CORE_LOAD_PATH", "src")]));
    let n = r.nwnx.unwrap_or_else(|| panic!("{:?}", r.notes));
    assert_eq!(n.binaries, here.join("src"));
    let paths: Vec<_> =
        n.server_env().into_iter().filter(|(k, _)| k == "NWNX_CORE_LOAD_PATH").collect();
    assert_eq!(paths, [("NWNX_CORE_LOAD_PATH".into(), here.join("src").into())]);
}

/// The loader's refusal of a library: NWNX not loaded where it is NWNX's
/// core (an error), another library left out where it isn't (a warning,
/// whether NWNX is on or not: the server runs, without that library).
#[test]
fn a_library_the_loader_refuses_is_told_by_what_it_is() {
    let refused = |path: &str| {
        format!(
            "ERROR: ld.so: object '{path}' from LD_PRELOAD cannot be preloaded (cannot open \
             shared object file): ignored."
        )
    };
    for nwnx in [true, false] {
        let core = console_note(&refused("/x/Binaries/NWNX_Core.so"), nwnx).unwrap();
        assert_eq!(core.level, Level::Error, "{}", core.text);
        assert!(core.text.starts_with("NWNX was not loaded"), "{}", core.text);
        let other = console_note(&refused("/usr/lib/libjemalloc.so"), nwnx).unwrap();
        assert_eq!(other.level, Level::Warning, "{}", other.text);
        assert!(other.text.contains("/usr/lib/libjemalloc.so"), "{}", other.text);
        assert!(!other.text.contains("NWNX was not"), "{}", other.text);
    }
}
