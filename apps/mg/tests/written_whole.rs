//! The files `mg` writes for its caller (`mg gff -o`, `mg tlk-export`,
//! `mg dialog-export`, `mg plugin pack`) are written whole or not at all:
//! beside their place and moved into it, never over the file that was
//! there, which a command stopped halfway would leave cut short.

use std::path::{Path, PathBuf};

use mg_core::ResType;
use mg_module::dialog::{Parent, add_node, new_dialog};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

/// A file at `path` with a second name: a write over the file itself
/// would show under the second name too, a file moved into its place
/// leaves the second name with what was there.
fn held(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(".held");
    let second = path.with_file_name(name);
    std::fs::write(path, b"as it was").unwrap();
    std::fs::hard_link(path, &second).unwrap();
    second
}

fn run(args: &[&str]) {
    let out = crate::mg_alone().args(args).output().unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// What was at `path` is as it was under its second name, the new file
/// is in its place, and nothing is left beside it.
fn moved_into_place(path: &Path, second: &Path) {
    assert_eq!(std::fs::read(second).unwrap(), b"as it was", "{}", path.display());
    assert_ne!(std::fs::read(path).unwrap(), b"as it was", "{}", path.display());
    let dir = path.parent().unwrap();
    let left: Vec<_> = (std::fs::read_dir(dir).unwrap().flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".moonglow-tmp"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn the_files_a_command_writes_are_moved_into_place() {
    let dir = mg_testkit::scratch_dir("mg-written-whole");
    let at = |name: &str| dir.join(name);
    let text = |p: &Path| p.to_str().unwrap().to_owned();

    // `mg gff -o`, both ways.
    let mut m = Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello.");
    m.set_gff(ResKey::parse("greet", ResType::DLG).unwrap(), &g).unwrap();
    let module = at("town.mod");
    m.save_as(&ModuleLocation::Archive(module.clone())).unwrap();
    std::fs::write(at("greet.dlg"), g.to_bytes().unwrap()).unwrap();
    let json = at("greet.json");
    let second = held(&json);
    run(&["gff", &text(&at("greet.dlg")), "-o", &text(&json)]);
    moved_into_place(&json, &second);
    let back = at("back.dlg");
    let second = held(&back);
    run(&["gff", &text(&json), "-o", &text(&back)]);
    moved_into_place(&back, &second);
    assert_eq!(std::fs::read(&back).unwrap(), g.to_bytes().unwrap());

    // `mg dialog-export`, a file and a folder of them.
    let ink = at("greet.ink");
    let second = held(&ink);
    run(&["dialog-export", &text(&module), "greet.dlg", &text(&ink)]);
    moved_into_place(&ink, &second);
    std::fs::create_dir_all(at("stories")).unwrap();
    let story = at("stories/greet.ink");
    let second = held(&story);
    run(&[
        "dialog-export",
        &text(&module),
        "greet.dlg",
        "--to",
        &text(&at("stories")),
        "--format",
        "ink",
        "--force",
    ]);
    moved_into_place(&story, &second);

    // `mg tlk-export`.
    std::fs::write(at("in.json"), r#"{"language": 0, "entries": [{"id": 0, "text": "One"}]}"#)
        .unwrap();
    run(&["tlk-import", &text(&at("custom.tlk")), &text(&at("in.json"))]);
    let out = at("out.json");
    let second = held(&out);
    run(&["tlk-export", &text(&at("custom.tlk")), &text(&out)]);
    moved_into_place(&out, &second);

    // `mg plugin pack`.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/mg-plugin/tests/fixtures/tag-conventions");
    let zip = at("tags.zip");
    let second = held(&zip);
    run(&["plugin", "pack", &text(&source), "-o", &text(&zip)]);
    moved_into_place(&zip, &second);
}
