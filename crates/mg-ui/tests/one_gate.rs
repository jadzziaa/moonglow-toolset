//! Every change to the open module goes through `Moonglow::apply` (edits,
//! undoable) or `Workspace::derive` (what is made from the module at save
//! or build): the warnings, the derived values, the hak list and the
//! revision that views watch all hang on that. Every dropdown is drawn
//! by `widgets`, so that every one takes the arrow keys. This reads the
//! sources, so that a new way round fails here instead of being found
//! later.

use std::path::Path;

/// The crate's sources, each without its tests.
fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            let code = text.split("#[cfg(test)]").next().unwrap_or_default().to_string();
            out.push((path.display().to_string(), code));
        }
    }
}

/// The crate's sources without their tests, as (file, code).
fn crate_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    files
}

/// Every dropdown is drawn by `widgets` (`widgets::dropdown`, the lists of
/// a table's rows), so that every one takes the arrow keys as a list of
/// choices does: an egui combo box made elsewhere would not.
#[test]
fn every_dropdown_goes_through_the_widgets() {
    let files = crate_sources();
    assert!(files.len() > 40, "the sources were found");
    let elsewhere: Vec<String> = files
        .iter()
        .filter(|(file, _)| !file.ends_with("src/widgets.rs"))
        .flat_map(|(file, code)| {
            code.lines()
                .enumerate()
                .filter(|(_, l)| l.contains("ComboBox") && !l.trim_start().starts_with("//"))
                .map(move |(n, l)| format!("{file}:{}: {}", n + 1, l.trim()))
        })
        .collect();
    assert!(
        elsewhere.is_empty(),
        "draw a dropdown with widgets::dropdown, not an egui ComboBox:\n{}",
        elsewhere.join("\n")
    );
}

#[test]
fn the_module_changes_only_through_the_one_gate() {
    let files = crate_sources();
    assert!(files.len() > 40, "the sources were found");
    let lines_with = |what: &str| -> Vec<String> {
        files
            .iter()
            .flat_map(|(file, code)| {
                code.lines()
                    .enumerate()
                    .filter(|(_, l)| l.contains(what) && !l.trim_start().starts_with("//"))
                    .map(move |(n, l)| format!("{file}:{}: {}", n + 1, l.trim()))
            })
            .collect()
    };
    // A command is applied in one place.
    let applied = lines_with("ws.apply(");
    assert!(
        applied.len() == 1 && applied[0].contains("lib.rs"),
        "apply commands with Moonglow::apply, not the workspace's:\n{}",
        applied.join("\n")
    );
    // The module is not written past the workspace.
    for direct in
        ["&mut ws.module", "ws.module.set", "ws.module.remove", "ws.module.adopt", ".module = "]
    {
        let found = lines_with(direct);
        assert!(found.is_empty(), "the module changed past the workspace:\n{}", found.join("\n"));
    }
}
