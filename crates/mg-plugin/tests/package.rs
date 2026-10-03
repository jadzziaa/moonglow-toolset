//! A plugin's archive: read, refused where it is not one or not safe, and
//! installed into a plugins folder.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mg_plugin::{
    Answer, Existing, Host, Level, MARKER, MAX_FILES, Package, Plugin, PluginError, Question,
    discover, from_archive, inspect, pack, remove,
};
use zip::write::SimpleFileOptions;

fn fixture(file: &str) -> Vec<u8> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tag-conventions");
    std::fs::read(dir.join(file)).unwrap()
}

/// A zip of these entries, as given (a name ending in `/` is a folder's).
fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in entries {
        if name.ends_with('/') {
            zip.add_directory(*name, options).unwrap();
        } else {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
    }
    zip.finish().unwrap().into_inner()
}

/// The tag-conventions plugin's files under `prefix`, with its manifest
/// as `manifest`.
fn plugin_under(prefix: &str, manifest: &[u8]) -> Vec<(String, Vec<u8>)> {
    vec![
        (format!("{prefix}plugin.cfg"), manifest.to_vec()),
        (format!("{prefix}main.luau"), fixture("main.luau")),
        (format!("{prefix}rules.luau"), fixture("rules.luau")),
    ]
}

fn zipped(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let entries: Vec<(&str, &[u8])> =
        entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    archive(&entries)
}

fn refusal(bytes: &[u8]) -> String {
    match Package::read(bytes) {
        Ok(p) => panic!("read as {}", p.manifest.id),
        Err(e) => e.to_string(),
    }
}

struct Quiet;

impl Host for Quiet {
    fn log(&self, _: Level, _: &str) {}
    fn ask(&self, _: &Question) -> Option<Answer> {
        None
    }
}

/// Everything under `dir`, as paths from it, sorted.
fn tree(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out);
            } else {
                out.push(path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn a_plugin_installs_from_its_archive_and_again_over_itself() {
    let plugins = mg_testkit::scratch_dir("plugin-install");
    let first = Package::read(&zipped(&plugin_under("", &fixture("plugin.cfg")))).unwrap();
    assert_eq!(first.manifest.id, "example.tag-conventions");
    assert_eq!(first.files().collect::<Vec<_>>(), ["main.luau", "plugin.cfg", "rules.luau"]);
    assert_eq!(first.existing(&plugins), Existing::Nothing);

    // Into a folder named by its id, with the mark of an install; none
    // of it runs, and it is a plugin like one copied there.
    let installed = first.install(&plugins, false).unwrap();
    assert_eq!(installed.dir, plugins.join("example.tag-conventions"));
    assert_eq!(
        tree(&plugins),
        [
            format!("example.tag-conventions/{MARKER}"),
            "example.tag-conventions/main.luau".to_string(),
            "example.tag-conventions/plugin.cfg".to_string(),
            "example.tag-conventions/rules.luau".to_string(),
        ]
    );
    assert_eq!(inspect(&installed, Rc::new(Quiet)).unwrap(), Vec::<String>::new());
    assert_eq!(discover(&plugins).len(), 1);

    // Again: only when told to replace it, and then all of it (a file
    // the new one lacks is gone).
    let manifest = String::from_utf8(fixture("plugin.cfg")).unwrap().replace("1.0.0", "1.1.0");
    let mut files = plugin_under("", manifest.as_bytes());
    files.retain(|(name, _)| name != "rules.luau");
    files[1].1 = b"local mg = require(\"@moonglow\")\nmg.command(\"fix-tags\", function() end)\n\
                   mg.command(\"promote\", function() end)\nmg.check(\"tag-case\", function() end)\n"
        .to_vec();
    files.push(("docs/README.md".into(), b"Notes.".to_vec()));
    let second = Package::read(&zipped(&files)).unwrap();
    assert_eq!(second.existing(&plugins), Existing::Installed { version: "1.0.0".into() });
    let e = second.install(&plugins, false).unwrap_err().to_string();
    assert_eq!(e, "Tag conventions 1.0.0 is installed already");
    assert!(plugins.join("example.tag-conventions/rules.luau").is_file());
    let installed = second.install(&plugins, true).unwrap();
    assert_eq!(installed.manifest.version, "1.1.0");
    assert_eq!(
        tree(&plugins),
        [
            format!("example.tag-conventions/{MARKER}"),
            "example.tag-conventions/docs/README.md".to_string(),
            "example.tag-conventions/main.luau".to_string(),
            "example.tag-conventions/plugin.cfg".to_string(),
        ]
    );
}

/// As zipping a folder makes it, or a repository's download: the plugin
/// in the one folder at the top. Hidden files and what is beside the
/// folder stay out.
#[test]
fn the_plugin_may_be_in_a_folder_of_the_archive() {
    let mut files = plugin_under("tag-conventions-main/", &fixture("plugin.cfg"));
    files.extend([
        ("README.md".to_string(), b"About the repository.".to_vec()),
        (".github/workflows/ci.yml".to_string(), b"on: push".to_vec()),
        ("__MACOSX/._plugin.cfg".to_string(), b"junk".to_vec()),
        ("tag-conventions-main/.git/config".to_string(), b"[core]".to_vec()),
        ("tag-conventions-main/.DS_Store".to_string(), b"junk".to_vec()),
        ("tag-conventions-main/lib/".to_string(), Vec::new()),
        ("tag-conventions-main/lib/names.luau".to_string(), b"return {}".to_vec()),
        // (As a Windows tool may write it.)
        ("tag-conventions-main\\lib\\more.luau".to_string(), b"return {}".to_vec()),
    ]);
    let package = Package::read(&zipped(&files)).unwrap();
    assert_eq!(
        package.files().collect::<Vec<_>>(),
        ["lib/more.luau", "lib/names.luau", "main.luau", "plugin.cfg", "rules.luau"]
    );
    let plugins = mg_testkit::scratch_dir("plugin-install-folder");
    package.install(&plugins, false).unwrap();
    assert!(plugins.join("example.tag-conventions/lib/names.luau").is_file());
    assert!(!plugins.join("example.tag-conventions/.git").exists());
}

#[test]
fn what_is_no_plugin_or_not_safe_is_refused() {
    let good = || plugin_under("", &fixture("plugin.cfg"));
    let with = |name: &str, data: &[u8]| {
        let mut files = good();
        files.push((name.to_string(), data.to_vec()));
        refusal(&zipped(&files))
    };
    let without = |name: &str| {
        let mut files = good();
        files.retain(|(n, _)| n != name);
        refusal(&zipped(&files))
    };

    assert_eq!(refusal(b"not an archive at all"), "it is not a zip archive");
    assert_eq!(refusal(b""), "it is not a zip archive");
    assert_eq!(without("plugin.cfg"), "it has no plugin.cfg at its top, or in a folder at its top");
    assert_eq!(without("main.luau"), "its entry main.luau is not in it");

    // Names that would leave the folder, or that a system cannot have.
    for (name, why) in [
        ("../evil.luau", "leads out of the plugin's folder"),
        ("lib/../../evil.luau", "leads out of the plugin's folder"),
        ("lib\\..\\..\\evil.luau", "leads out of the plugin's folder"),
        ("lib//evil.luau", "leads out of the plugin's folder"),
        ("/etc/evil.luau", "is an absolute path"),
        ("\\evil.luau", "is an absolute path"),
        ("c:evil.luau", "has a character a file's name cannot have"),
        ("lib/a\nb.luau", "has a character a file's name cannot have"),
        ("nul.luau", "has a name Windows keeps for a device"),
        ("lib/COM1", "has a name Windows keeps for a device"),
        ("trailing.", "ends with a dot or a space"),
        ("a/b/c/d/e/f/g/h/i.luau", "is in too deep a folder"),
    ] {
        let e = with(name, b"x");
        assert!(e.contains(why), "{name:?}: {e}");
    }
    // (Even where it would be skipped.)
    assert!(with(".hidden/../../evil", b"x").contains("leads out"));
    assert!(with("../", b"").contains("leads out"));

    assert_eq!(with("Main.luau", b"x"), "it has \"Main.luau\" twice");
    assert_eq!(with("rules.luau/more.luau", b"x"), "\"rules.luau\" is a file and a folder in it");

    // Several plugins, a manifest for another API, one that is not text.
    let mut two = plugin_under("one/", &fixture("plugin.cfg"));
    two.extend(plugin_under("two/", &fixture("plugin.cfg")));
    assert_eq!(
        refusal(&zipped(&two)),
        "it holds several plugins (one, two): install each from an archive of its own"
    );
    let other = String::from_utf8(fixture("plugin.cfg")).unwrap().replace("\"0.1\"", "\"7.0\"");
    let e = refusal(&zipped(&plugin_under("", other.as_bytes())));
    assert!(e.contains("it was written for plugin API 7.0"), "{e}");
    assert_eq!(
        refusal(&zipped(&plugin_under("", b"[plugin]\nname = \"\xff\"\n"))),
        "its plugin.cfg is not text (UTF-8)"
    );

    // An id that some system cannot have as a folder's name.
    for id in ["nul", "com1.tags", "tags."] {
        let named = String::from_utf8(fixture("plugin.cfg"))
            .unwrap()
            .replace("example.tag-conventions", id);
        assert_eq!(
            refusal(&zipped(&plugin_under("", named.as_bytes()))),
            format!("its id {id:?} cannot name a folder")
        );
    }

    // Too many files, and too much data (which is small while packed).
    let mut many = good();
    for i in 0..MAX_FILES {
        many.push((format!("lib/f{i}.luau"), b"return 1".to_vec()));
    }
    assert_eq!(refusal(&zipped(&many)), format!("it has over {MAX_FILES} files"));
    let bomb = zipped(&[good(), vec![("big.bin".to_string(), vec![0u8; 17 << 20])]].concat());
    assert!(bomb.len() < 100_000, "{}", bomb.len());
    assert_eq!(refusal(&bomb), "it unpacks to over 16 MB");

    // A link.
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in good() {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&data).unwrap();
    }
    zip.add_symlink("passwords", "/etc/passwd", SimpleFileOptions::default()).unwrap();
    let linked = zip.finish().unwrap().into_inner();
    assert_eq!(refusal(&linked), "the entry \"passwords\" is a link");
}

/// An archive cut short or damaged anywhere reads as an error, or as a
/// plugin still: never a panic.
#[test]
fn a_damaged_archive_is_an_error() {
    let whole = zipped(&plugin_under("", &fixture("plugin.cfg")));
    assert!(Package::read(&whole).is_ok());
    for cut in 0..whole.len() {
        assert!(Package::read(&whole[..cut]).is_err(), "cut at {cut}");
    }
    for at in 0..whole.len() {
        let mut damaged = whole.clone();
        damaged[at] ^= 0xff;
        let _ = Package::read(&damaged);
        damaged[at] = 0;
        let _ = Package::read(&damaged);
    }
}

/// A folder that was not installed from an archive (an author's own, a
/// plugin copied in by hand) is never replaced.
#[test]
fn a_folder_not_installed_from_an_archive_is_left_alone() {
    let package = Package::read(&zipped(&plugin_under("", &fixture("plugin.cfg")))).unwrap();
    let copy = |to: &Path| {
        std::fs::create_dir_all(to).unwrap();
        for file in ["plugin.cfg", "main.luau", "rules.luau"] {
            std::fs::write(to.join(file), fixture(file)).unwrap();
        }
        std::fs::write(to.join("notes.txt"), "my work").unwrap();
    };
    // Its id in a folder of another name.
    let plugins = mg_testkit::scratch_dir("plugin-install-foreign");
    let mine = plugins.join("my-tags");
    copy(&mine);
    assert_eq!(package.existing(&plugins), Existing::Foreign { dir: mine.clone() });
    for replace in [false, true] {
        let e = package.install(&plugins, replace).unwrap_err();
        assert!(
            matches!(&e, PluginError::Package(m) if m.contains("was not installed from a file"))
        );
    }
    assert_eq!(std::fs::read_to_string(mine.join("notes.txt")).unwrap(), "my work");
    assert_eq!(discover(&plugins).len(), 1);

    // In the folder the install would use.
    let plugins = mg_testkit::scratch_dir("plugin-install-foreign-same");
    let same = plugins.join("example.tag-conventions");
    copy(&same);
    assert_eq!(package.existing(&plugins), Existing::Foreign { dir: same.clone() });
    assert!(package.install(&plugins, true).is_err());
    assert_eq!(std::fs::read_to_string(same.join("notes.txt")).unwrap(), "my work");
}

/// What an install leaves half done is hidden, no plugin, and cleared by
/// the next one.
#[test]
fn an_install_under_way_is_no_plugin() {
    let plugins = mg_testkit::scratch_dir("plugin-install-stale");
    let stale = plugins.join(".installing-example.tag-conventions");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("plugin.cfg"), fixture("plugin.cfg")).unwrap();
    std::fs::write(stale.join("main.luau"), fixture("main.luau")).unwrap();
    std::fs::write(stale.join("leftover.luau"), "old").unwrap();
    assert!(discover(&plugins).is_empty());

    let package = Package::read(&zipped(&plugin_under("", &fixture("plugin.cfg")))).unwrap();
    assert_eq!(package.existing(&plugins), Existing::Nothing);
    let installed = package.install(&plugins, false).unwrap();
    assert!(!stale.exists());
    assert!(!installed.dir.join("leftover.luau").exists());
    let found: Vec<Plugin> = discover(&plugins).into_iter().map(Result::unwrap).collect();
    assert_eq!(found, [installed]);
}

/// A plugin's folder packed is an archive that installs as the same
/// plugin: without its hidden files, and the same bytes each time.
#[test]
fn a_packed_plugin_installs_as_itself() {
    let dir = mg_testkit::scratch_dir("plugin-pack");
    let source = dir.join("work/my-tags");
    std::fs::create_dir_all(source.join("lib")).unwrap();
    std::fs::create_dir_all(source.join(".git")).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::write(source.join(file), fixture(file)).unwrap();
    }
    std::fs::write(source.join("lib/names.luau"), "return {}").unwrap();
    std::fs::write(source.join(".git/config"), "[core]").unwrap();
    std::fs::write(source.join(".luaurc"), "{}").unwrap();
    std::fs::write(source.join(MARKER), "installed").unwrap();
    let plugin = Plugin::load(&source).unwrap();

    let bytes = pack(&plugin).unwrap();
    assert_eq!(pack(&plugin).unwrap(), bytes);
    let package = Package::read(&bytes).unwrap();
    assert_eq!(package.manifest, plugin.manifest);
    assert_eq!(
        package.files().collect::<Vec<_>>(),
        ["lib/names.luau", "main.luau", "plugin.cfg", "rules.luau"]
    );
    let plugins = dir.join("plugins");
    let installed = package.install(&plugins, false).unwrap();
    assert_eq!(inspect(&installed, Rc::new(Quiet)).unwrap(), Vec::<String>::new());
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        assert_eq!(std::fs::read(installed.dir.join(file)).unwrap(), fixture(file), "{file}");
    }

    // A file the installer would refuse is refused here.
    std::fs::write(source.join("aux.luau"), "return 1").unwrap();
    let e = pack(&plugin).unwrap_err().to_string();
    assert!(e.contains("\"aux.luau\" has a name Windows keeps for a device"), "{e}");
}

/// An archive's code is loaded as it is in the archive, nothing unpacked:
/// what it registers is compared with its manifest as a folder's is.
#[test]
fn an_archive_s_code_is_checked_without_unpacking() {
    // (Its main.luau requires rules.luau: from the archive too.)
    let good = Package::read(&zipped(&plugin_under("", &fixture("plugin.cfg")))).unwrap();
    assert_eq!(good.inspect(Rc::new(Quiet)).unwrap(), Vec::<String>::new());

    let broken = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/broken");
    let files: Vec<(String, Vec<u8>)> = ["plugin.cfg", "main.luau"]
        .iter()
        .map(|f| (format!("broken/{f}"), std::fs::read(broken.join(f)).unwrap()))
        .collect();
    let package = Package::read(&zipped(&files)).unwrap();
    let from_folder = inspect(&Plugin::load(&broken).unwrap(), Rc::new(Quiet)).unwrap();
    assert_eq!(from_folder.len(), 3);
    assert_eq!(package.inspect(Rc::new(Quiet)).unwrap(), from_folder);

    // Code that does not load is an error that names the script.
    let mut files = plugin_under("", &fixture("plugin.cfg"));
    files[2].1 = b"this is not Luau".to_vec();
    let e = Package::read(&zipped(&files)).unwrap().inspect(Rc::new(Quiet)).unwrap_err();
    assert!(e.to_string().contains("rules.luau"), "{e}");
}

/// Remove takes away what Install from File put there, and nothing else.
#[test]
fn only_what_was_installed_from_an_archive_is_removed() {
    let plugins = mg_testkit::scratch_dir("plugin-remove");
    let package = Package::read(&zipped(&plugin_under("", &fixture("plugin.cfg")))).unwrap();
    let installed = package.install(&plugins, false).unwrap();
    assert!(from_archive(&installed.dir));
    remove(&installed.dir).unwrap();
    assert!(!installed.dir.exists());
    assert!(tree(&plugins).is_empty(), "{:?}", tree(&plugins));
    assert_eq!(package.existing(&plugins), Existing::Nothing);

    // A folder put there by hand is refused, and stays.
    let mine = plugins.join("my-tags");
    std::fs::create_dir_all(&mine).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::write(mine.join(file), fixture(file)).unwrap();
    }
    assert!(!from_archive(&mine));
    let e = remove(&mine).unwrap_err().to_string();
    assert!(e.ends_with("was not installed from a file: delete its folder to remove it"), "{e}");
    assert_eq!(tree(&plugins).len(), 3);

    // A link to a plugin kept elsewhere: the link goes, the plugin stays.
    #[cfg(unix)]
    {
        let elsewhere = mg_testkit::scratch_dir("plugin-remove-elsewhere");
        let kept = package.install(&elsewhere, false).unwrap();
        let link = plugins.join("linked");
        std::os::unix::fs::symlink(&kept.dir, &link).unwrap();
        assert_eq!(discover(&plugins).len(), 2);
        remove(&link).unwrap();
        assert!(!link.exists());
        assert!(kept.dir.join("main.luau").is_file());
    }
}
