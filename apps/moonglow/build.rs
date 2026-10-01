//! On Windows, the executable's icon and version information.

fn main() {
    println!("cargo:rerun-if-changed=../../packaging/icons/moonglow.ico");
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
