//! Keeps a test server up for 25 seconds and prints its network log, to
//! check that test servers never register on the public server list.
//!
//!     cargo run -p mg-testkit --example server_privacy
fn main() {
    let root = mg_testkit::nwn_root().expect("no game install");
    let dir = mg_testkit::scratch_dir("server_privacy");
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), dir.join("modules/chess.mod"))
        .unwrap();
    let run = mg_testkit::engine::run_server(
        &root,
        &dir,
        "chess",
        "NEVER_LOGGED",
        std::time::Duration::from_secs(25),
    )
    .unwrap();
    assert!(!run.finished);
    let engine_log =
        std::fs::read_to_string(dir.join("logs.0/nwengineLog.txt")).unwrap_or_default();
    for l in engine_log
        .lines()
        .filter(|l| l.contains("Network") || l.contains("Masterserver") || l.contains("listing"))
    {
        println!("{l}");
    }
    let listed = engine_log.contains("visible on public listing");
    println!("publicly listed: {listed}");
    assert!(!listed, "the test server registered on the public list");
}
