//! On Windows, the executable's icon and version information; and the key
//! the releases' checksums are signed with, where the owner has committed
//! it (`packaging/minisign.pub`, `packaging/README.md`).

fn main() {
    println!("cargo:rerun-if-changed=../../packaging/icons/moonglow.ico");
    update_key();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../packaging/icons/moonglow.ico")
        .set("ProductName", "Moonglow Toolset")
        .set("FileDescription", "Moonglow Toolset")
        .set("LegalCopyright", "GPL-3.0-only");
    // Without a resource compiler (cross builds), the program still builds,
    // only without its icon.
    if let Err(e) = res.compile() {
        println!("cargo:warning=no Windows resources (icon, version): {e}");
    }
}

/// `packaging/minisign.pub` (as `minisign -G` writes it: an untrusted
/// comment, then the key) given to the program as `MOONGLOW_UPDATE_KEY`,
/// its key's line. Without the file the build has no key, and installs no
/// update (the window links the release's page).
fn update_key() {
    // (The folder, so that the file is seen as it appears.)
    println!("cargo:rerun-if-changed=../../packaging");
    let Ok(text) = std::fs::read_to_string("../../packaging/minisign.pub") else { return };
    let key = text
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
        .unwrap_or_default();
    let base64 = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=');
    // (A minisign public key is 56 characters of base64.)
    assert!(
        key.len() == 56 && key.chars().all(base64),
        "packaging/minisign.pub holds no minisign public key"
    );
    println!("cargo:rustc-env=MOONGLOW_UPDATE_KEY={key}");
}
