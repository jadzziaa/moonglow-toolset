//! What the crate bundles reaches the packages' `THIRD-PARTY-LICENSES.txt`:
//! `packaging/third_party_licenses.py`, which every package runs, is run
//! here, and its notices of the icons (Lucide's ISC license) and the fonts
//! (the Ubuntu Font Licence, for Ubuntu Light and Ubuntu Bold) are read
//! back.

/// The file the script writes, as notice blocks: each block's users (one a
/// line) and its text.
fn notices(text: &str) -> Vec<(Vec<String>, String)> {
    let rule = format!("\n{}\n", "=".repeat(78));
    let line = format!("{}\n\n", "-".repeat(78));
    text.split(&rule)
        .skip(1)
        .filter_map(|block| {
            let (users, text) = block.split_once(&line)?;
            Some((users.lines().map(str::to_string).collect(), text.trim().to_string()))
        })
        .collect()
}

#[test]
fn the_packages_notices_name_the_icons_and_the_fonts() {
    let dir = mg_testkit::scratch_dir("uikit-licenses");
    let out = dir.join("THIRD-PARTY-LICENSES.txt");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let run = std::process::Command::new("python3")
        .arg("packaging/third_party_licenses.py")
        .arg("--output")
        .arg(&out)
        .current_dir(&root)
        .output()
        .expect("python3 runs");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let text = std::fs::read_to_string(&out).unwrap();
    let head = text.split("It is built from these crates").next().unwrap().replace('\n', " ");
    assert!(head.contains("in the manner of Lucide"), "{head}");
    assert!(head.contains("Ubuntu Light") && head.contains("Ubuntu Bold"), "{head}");

    let notices = notices(&text);
    let lucide = include_str!("../../icons/LICENSE-lucide.txt").trim();
    assert!(
        notices.iter().any(|(users, t)| t == lucide
            && users.contains(&"mg-uikit (icons/LICENSE-lucide.txt)".to_string())),
        "no Lucide notice from mg-uikit's icons"
    );
    let ufl = include_str!("../../fonts/UFL.txt").trim();
    let (users, _) =
        notices.iter().find(|(_, t)| t == ufl).expect("the Ubuntu Font Licence's text");
    // (Ubuntu Bold is the crate's; Ubuntu Light egui's, in its fonts.)
    assert!(users.contains(&"mg-uikit (fonts/UFL.txt)".to_string()), "{users:?}");
    assert!(
        users.iter().any(|u| u.starts_with("epaint_default_fonts ") && u.ends_with(" (UFL.txt)")),
        "{users:?}"
    );
}
