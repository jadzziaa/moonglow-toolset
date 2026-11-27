use super::*;
use crate::Module;
use mg_core::ResRef;
use mg_resman::ResKey;

/// A fake NWNX home in a scratch folder: `Binaries/` with empty `.so`
/// files for `plugins` (Core's always), `NWScript/` with two includes.
fn fake_home(dir: &Path, plugins: &[&str]) -> PathBuf {
    let home = dir.join("nwnx");
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
    assert!(r.errors()[0].contains("NWNX_TDOD_SKIP switches the plugin TDOD on"), "{:?}", r.notes);
    assert!(r.errors()[0].contains("NWNX_TDOD.so"), "{:?}", r.notes);
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
/// plugin that failed to load a warning. (The fixtures are NWNX's own
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
    assert_eq!(line(skip), Some(Level::Info));
    assert_eq!(line("Server: Module loaded"), None);
    assert_eq!(console_note("NWNX: Expected build 1.2, got build 3.4.", false), None);
}
